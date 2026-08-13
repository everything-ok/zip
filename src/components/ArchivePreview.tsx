import { useMemo, useState } from "react";
import { File, Folder, Lock, Search, ChevronDown, ChevronRight, Loader2, ArrowRight, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { EntryDto } from "../lib/types";
import { formatBytes, basename, compressionRatio, formatTime } from "../lib/format";
import { Badge, Button } from "./ui";

const PREVIEW_LIMIT = 500;

interface TreeNode {
  name: string;
  full: string;
  entry?: EntryDto;
  children: Map<string, TreeNode>;
}

/** 把平铺路径构造成目录树。 */
function buildTree(entries: EntryDto[]): TreeNode {
  const root: TreeNode = { name: "", full: "", children: new Map() };
  for (const entry of entries) {
    const parts = entry.path.replace(/\\/g, "/").split("/").filter(Boolean);
    let node = root;
    let acc = "";
    for (let i = 0; i < parts.length; i++) {
      acc = acc ? `${acc}/${parts[i]}` : parts[i];
      let child = node.children.get(parts[i]);
      if (!child) {
        child = { name: parts[i], full: acc, children: new Map() };
        node.children.set(parts[i], child);
      }
      if (i === parts.length - 1) {
        child.entry = entry;
      }
      node = child;
    }
  }
  return root;
}

export function ArchivePreview({
  path,
  format,
  entries,
  loading = false,
  onConvert,
  onTest,
}: {
  path: string;
  format?: string;
  entries: EntryDto[];
  loading?: boolean;
  onConvert?: () => void;
  onTest?: () => void;
}) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState(false);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const filtered = useMemo(() => {
    if (!query.trim()) return entries;
    const q = query.toLowerCase();
    return entries.filter((e) => e.path.toLowerCase().includes(q));
  }, [entries, query]);

  const total = useMemo(
    () => entries.reduce((s, e) => s + (e.is_dir ? 0 : e.size), 0),
    [entries]
  );
  const compressedTotal = useMemo(
    () => entries.reduce((s, e) => s + (e.is_dir ? 0 : e.compressed_size), 0),
    [entries]
  );
  const encrypted = entries.some((e) => e.is_encrypted);

  // 平铺视图（搜索时或展开时使用）：带压缩率与修改时间列。
  const flatRows = useMemo(() => {
    const rows = filtered.slice(0, expanded ? filtered.length : PREVIEW_LIMIT);
    return rows;
  }, [filtered, expanded]);

  // 树形视图（无搜索时）：分组展示目录。
  const tree = useMemo(() => buildTree(filtered), [filtered]);

  const toggleCollapse = (full: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(full)) next.delete(full);
      else next.add(full);
      return next;
    });
  };

  const renderTree = (node: TreeNode, depth: number): React.ReactNode => {
    const children = [...node.children.values()].sort((a, b) => {
      // 目录优先，再按名称。
      const aDir = a.children.size > 0 || a.entry?.is_dir;
      const bDir = b.children.size > 0 || b.entry?.is_dir;
      if (aDir !== bDir) return aDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    return children.map((child) => {
      const isDir = child.children.size > 0 || child.entry?.is_dir;
      const isCollapsed = collapsed.has(child.full);
      const pad = { paddingLeft: `${depth * 16 + 12}px` };
      return (
        <div key={child.full}>
          <div
            className="flex items-center gap-2 border-b border-zinc-100 py-1.5 pr-4 hover:bg-zinc-50 dark:border-zinc-800 dark:hover:bg-zinc-800/40"
            style={pad}
          >
            {isDir ? (
              <button
                onClick={() => toggleCollapse(child.full)}
                className="shrink-0 text-zinc-400 hover:text-zinc-600"
              >
                {isCollapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
              </button>
            ) : (
              <span className="w-[13px] shrink-0" />
            )}
            {isDir ? (
              <Folder size={14} className="shrink-0 text-indigo-400" />
            ) : (
              <File size={14} className="shrink-0 text-zinc-400" />
            )}
            <span className="min-w-0 flex-1 truncate text-zinc-700 dark:text-zinc-300">
              {child.name}
            </span>
            {child.entry?.is_encrypted && <Lock size={11} className="shrink-0 text-amber-500" />}
            <span className="shrink-0 font-mono text-xs text-zinc-500">
              {isDir ? "-" : formatBytes(child.entry?.size ?? 0)}
            </span>
            <span className="hidden shrink-0 font-mono text-xs text-zinc-400 sm:inline">
              {isDir ? "-" : compressionRatio(child.entry?.size ?? 0, child.entry?.compressed_size ?? 0)}
            </span>
            <span className="hidden shrink-0 font-mono text-xs text-zinc-400 md:inline">
              {formatTime(child.entry?.modified ?? null)}
            </span>
          </div>
          {isDir && !isCollapsed && renderTree(child, depth + 1)}
        </div>
      );
    });
  };

  const overLimit = filtered.length > PREVIEW_LIMIT;

  return (
    <div className="flex flex-col overflow-hidden rounded-xl border border-zinc-200 bg-white dark:border-zinc-800 dark:bg-zinc-900">
      <div className="flex items-center gap-2 border-b border-zinc-200 px-4 py-3 dark:border-zinc-800">
        <span className="truncate text-sm font-medium text-zinc-900 dark:text-zinc-100">
          {basename(path)}
        </span>
        {format && <Badge color="indigo">{format.toUpperCase()}</Badge>}
        {encrypted && (
          <Badge color="amber">
            <Lock size={11} className="mr-1" />
            {t("preview.encrypted")}
          </Badge>
        )}
        <span className="ml-auto flex items-center gap-1 text-xs text-zinc-500">
          {onConvert && (
            <Button variant="ghost" onClick={onConvert} className="px-2 py-0.5 text-xs" title={t("convert.title")}>
              <ArrowRight size={13} /> {t("convert.title")}
            </Button>
          )}
          {onTest && (
            <Button variant="ghost" onClick={onTest} className="px-2 py-0.5 text-xs" title={t("test.title")}>
              <ShieldCheck size={13} /> {t("test.title")}
            </Button>
          )}
          {t("preview.items", { count: entries.length })}
        </span>
      </div>
      {/* 搜索框 */}
      {entries.length > 0 && (
        <div className="border-b border-zinc-100 px-4 py-2 dark:border-zinc-800">
          <div className="flex items-center gap-2 rounded-lg bg-zinc-100 px-2.5 py-1.5 dark:bg-zinc-800">
            <Search size={14} className="shrink-0 text-zinc-400" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("preview.search")}
              className="min-w-0 flex-1 bg-transparent text-sm text-zinc-800 outline-none placeholder:text-zinc-400 dark:text-zinc-100"
            />
          </div>
        </div>
      )}
      <div className="max-h-64 overflow-auto">
        {loading ? (
          <div className="flex items-center justify-center gap-2 px-4 py-10 text-sm text-zinc-400">
            <Loader2 size={16} className="animate-spin" />
            <span>{t("preview.loading")}</span>
          </div>
        ) : entries.length === 0 ? (
          <div className="px-4 py-10 text-center text-sm text-zinc-400">
            {t("preview.empty")}
          </div>
        ) : filtered.length === 0 ? (
          <div className="px-4 py-10 text-center text-sm text-zinc-400">
            {t("preview.noMatch")}
          </div>
        ) : query.trim() ? (
          // 搜索结果用平铺表，带完整列。
          <div>
            <div className="grid grid-cols-[1fr_auto_auto_auto] gap-2 border-b border-zinc-200 px-4 py-1 text-xs font-medium text-zinc-400 dark:border-zinc-800">
              <span>{t("preview.colName")}</span>
              <span className="w-16 text-right">{t("preview.colSize")}</span>
              <span className="hidden w-12 text-right sm:block">{t("preview.colRatio")}</span>
              <span className="hidden w-32 text-right md:block">{t("preview.colModified")}</span>
            </div>
            {flatRows.map((e) => (
              <div
                key={e.path}
                className="grid grid-cols-[1fr_auto_auto_auto] items-center gap-2 border-b border-zinc-100 px-4 py-1.5 last:border-0 dark:border-zinc-800"
              >
                <span className="flex min-w-0 items-center gap-2">
                  {e.is_dir ? (
                    <Folder size={14} className="shrink-0 text-indigo-400" />
                  ) : (
                    <File size={14} className="shrink-0 text-zinc-400" />
                  )}
                  <span className="truncate text-zinc-700 dark:text-zinc-300">{e.path}</span>
                  {e.is_encrypted && <Lock size={11} className="shrink-0 text-amber-500" />}
                </span>
                <span className="w-16 text-right font-mono text-xs text-zinc-500">
                  {e.is_dir ? "-" : formatBytes(e.size)}
                </span>
                <span className="hidden w-12 text-right font-mono text-xs text-zinc-400 sm:block">
                  {compressionRatio(e.size, e.compressed_size)}
                </span>
                <span className="hidden w-32 text-right font-mono text-xs text-zinc-400 md:block">
                  {formatTime(e.modified)}
                </span>
              </div>
            ))}
          </div>
        ) : (
          <div>{renderTree(tree, 0)}</div>
        )}
      </div>
      {overLimit && !query.trim() && !expanded && (
        <button
          onClick={() => setExpanded(true)}
          className="border-t border-zinc-200 px-4 py-2 text-center text-xs text-indigo-500 hover:bg-indigo-50 dark:border-zinc-800 dark:hover:bg-indigo-950/30"
        >
          {t("preview.showAll", { count: filtered.length })}
        </button>
      )}
      <div className="border-t border-zinc-200 px-4 py-2 text-xs text-zinc-500 dark:border-zinc-800">
        {t("preview.total")}: {formatBytes(total)}
        {compressedTotal > 0 && (
          <span className="ml-2 text-zinc-400">
            · {t("preview.compressed")}: {formatBytes(compressedTotal)} (
            {compressionRatio(total, compressedTotal)})
          </span>
        )}
      </div>
    </div>
  );
}
