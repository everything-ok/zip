//! 分卷串联读取器：把 `.7z.001/.002/...` 或 `.zip.001/.002/...` 串联成单个 `Read + Seek` 流。

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// 分卷串联读取器。按序打开 `.NNN` 分卷文件，对外暴露为连续字节流。
pub struct SplitReader {
    /// 分卷文件路径列表（首卷在前）。
    volumes: Vec<PathBuf>,
    /// 每卷的字节长度（缓存，避免反复 stat）。
    volume_sizes: Vec<u64>,
    /// 当前卷索引。
    current_volume: usize,
    /// 当前卷内的文件句柄。
    current_file: Option<File>,
    /// 当前卷内偏移（从卷起始算）。
    current_offset_in_volume: u64,
    /// 全局偏移（从流起始算）。
    global_offset: u64,
}

impl SplitReader {
    /// 从首卷路径构造。自动扫描同目录下后续分卷。
    ///
    /// 首卷路径形如 `xxx.7z.001` 或 `xxx.zip.001`。
    /// 扫描 `xxx.7z.002`, `xxx.7z.003`, ... 直到文件不存在。
    pub fn from_first_volume(first: &Path) -> io::Result<Self> {
        let mut volumes = vec![first.to_path_buf()];
        let dir = first.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "no parent directory")
        })?;
        let orig_name = first
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"))?;

        // 找到数字后缀的位置：xxx.7z.001 → base = "xxx.7z.", num = "001"
        let lower = orig_name.to_ascii_lowercase();
        let (_base, num_str) = find_numeric_suffix(&lower)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not a split archive first volume"))?;

        // 首卷编号必须为 1
        if num_str.parse::<u32>().unwrap_or(0) != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "first volume number must be 1",
            ));
        }

        let width = num_str.len();
        let base_orig = &orig_name[..orig_name.len() - width];

        // 扫描后续分卷
        let mut n = 2u32;
        loop {
            let vol_name = format!("{}{:0width$}", base_orig, n, width = width);
            let vol_path = dir.join(&vol_name);
            if vol_path.exists() {
                volumes.push(vol_path);
                n += 1;
            } else {
                break;
            }
        }

        // 缓存每卷大小
        let mut volume_sizes = Vec::with_capacity(volumes.len());
        for vol in &volumes {
            let size = std::fs::metadata(vol)?.len();
            volume_sizes.push(size);
        }

        let first_file = File::open(&volumes[0])?;

        Ok(Self {
            volumes,
            volume_sizes,
            current_volume: 0,
            current_file: Some(first_file),
            current_offset_in_volume: 0,
            global_offset: 0,
        })
    }

    /// 从 ZIP 传统分卷构造（`.z01/.z02/.../.zip`）。
    ///
    /// `zip_path` 是最后的 `.zip` 文件（含中央目录），`z01_path` 是第一个数据分卷。
    /// 调用方需提供 `.z01` 路径；本函数扫描 `.z02`, `.z03`, ... 直到不存在。
    pub fn from_zip_classic_split(z01_path: &Path, zip_path: &Path) -> io::Result<Self> {
        let dir = z01_path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "no parent directory")
        })?;
        let orig_name = z01_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"))?;

        // .z01 → base = "xxx.", num = "01"
        let lower = orig_name.to_ascii_lowercase();
        let (_base, num_str) = find_z_suffix(&lower)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not a .zNN split volume"))?;

        let width = num_str.len();
        let base_orig = &orig_name[..orig_name.len() - width - 2]; // 去掉 ".z01"

        let mut volumes = vec![z01_path.to_path_buf()];
        let mut n = 2u32;
        loop {
            let vol_name = format!("{}{}.z{:0width$}", base_orig, "", n, width = width);
            let vol_path = dir.join(&vol_name);
            if vol_path.exists() {
                volumes.push(vol_path);
                n += 1;
            } else {
                break;
            }
        }
        // 最后加 .zip（含中央目录）
        volumes.push(zip_path.to_path_buf());

        let mut volume_sizes = Vec::with_capacity(volumes.len());
        for vol in &volumes {
            let size = std::fs::metadata(vol)?.len();
            volume_sizes.push(size);
        }

        let first_file = File::open(&volumes[0])?;

        Ok(Self {
            volumes,
            volume_sizes,
            current_volume: 0,
            current_file: Some(first_file),
            current_offset_in_volume: 0,
            global_offset: 0,
        })
    }

    /// 全部分卷的总字节长度。
    pub fn total_size(&self) -> u64 {
        self.volume_sizes.iter().sum()
    }

    /// 打开指定卷并定位到卷内偏移。
    fn open_volume_at(&mut self, vol_idx: usize, offset_in_vol: u64) -> io::Result<()> {
        if vol_idx >= self.volumes.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "volume index out of range"));
        }
        let mut f = File::open(&self.volumes[vol_idx])?;
        f.seek(SeekFrom::Start(offset_in_vol))?;
        self.current_volume = vol_idx;
        self.current_file = Some(f);
        self.current_offset_in_volume = offset_in_vol;
        Ok(())
    }

    /// 计算全局偏移对应的卷索引与卷内偏移。
    fn locate(&self, global_pos: u64) -> Option<(usize, u64)> {
        let mut acc: u64 = 0;
        for (i, &size) in self.volume_sizes.iter().enumerate() {
            if global_pos < acc + size {
                return Some((i, global_pos - acc));
            }
            acc += size;
        }
        // 恰好在末尾
        if global_pos == acc {
            let last = self.volume_sizes.len() - 1;
            Some((last, self.volume_sizes[last]))
        } else {
            None
        }
    }
}

impl Read for SplitReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let f = self.current_file.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "no open volume file")
        })?;

        let n = f.read(buf)?;
        if n == 0 {
            // 当前卷读完，尝试切到下一卷
            if self.current_volume + 1 < self.volumes.len() {
                self.open_volume_at(self.current_volume + 1, 0)?;
                self.global_offset += 0; // global_offset 不变，由 seek 管理
                // 递归读下一卷
                return self.read(buf);
            }
            // 所有卷都读完
            return Ok(0);
        }

        self.current_offset_in_volume += n as u64;
        self.global_offset += n as u64;
        Ok(n)
    }
}

impl Seek for SplitReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(delta) => self.global_offset as i64 + delta,
            SeekFrom::End(delta) => self.total_size() as i64 + delta,
        };

        if target < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid seek to negative position",
            ));
        }

        let global_pos = target as u64;
        let (vol_idx, offset_in_vol) = self
            .locate(global_pos)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek position out of range"))?;

        // 若已在目标卷且偏移一致，直接 seek 卷内
        if vol_idx == self.current_volume {
            if let Some(ref mut f) = self.current_file {
                f.seek(SeekFrom::Start(offset_in_vol))?;
                self.current_offset_in_volume = offset_in_vol;
                self.global_offset = global_pos;
                return Ok(global_pos);
            }
        }

        // 切卷
        self.open_volume_at(vol_idx, offset_in_vol)?;
        self.global_offset = global_pos;
        Ok(global_pos)
    }
}

/// 找到文件名末尾的数字后缀：`xxx.7z.001` → `("xxx.7z.", "001")`
fn find_numeric_suffix(lower: &str) -> Option<(&str, &str)> {
    // 从末尾找连续数字
    let mut end = lower.len();
    while end > 0 && lower.as_bytes().get(end - 1)?.is_ascii_digit() {
        end -= 1;
    }
    if end == lower.len() || end == 0 {
        return None;
    }
    let num_str = &lower[end..];
    // base 必须以 '.' 结尾（分卷分隔符）
    let base = &lower[..end];
    if !base.ends_with('.') {
        return None;
    }
    Some((base, num_str))
}

/// 找到 `.zNN` 后缀：`xxx.z01` → `("xxx", "01")`
fn find_z_suffix(lower: &str) -> Option<(&str, &str)> {
    let dot_z = lower.rfind(".z")?;
    let after = &lower[dot_z + 2..];
    if after.chars().all(|c| c.is_ascii_digit()) && !after.is_empty() {
        Some((&lower[..dot_z], after))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_numeric_suffix() {
        assert_eq!(
            find_numeric_suffix("archive.7z.001"),
            Some(("archive.7z.", "001"))
        );
        assert_eq!(
            find_numeric_suffix("archive.zip.001"),
            Some(("archive.zip.", "001"))
        );
        assert_eq!(find_numeric_suffix("archive.7z"), None);
        assert_eq!(find_numeric_suffix("archive.rar"), None);
    }

    #[test]
    fn test_find_z_suffix() {
        assert_eq!(find_z_suffix("archive.z01"), Some(("archive", "01")));
        assert_eq!(find_z_suffix("archive.z1"), Some(("archive", "1")));
        assert_eq!(find_z_suffix("archive.zip"), None);
    }
}
