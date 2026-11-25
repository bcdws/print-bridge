/**
 * TdPrint 发行版界面定制。集中在本文件，App.vue 等上游文件只保留少量引用，便于合并上游更新。
 */
import { invoke } from '@tauri-apps/api/core';

/** 远程任务 Tab（以及状态检测里的远程任务分组）暂不开放。 */
export const TDPRINT_SHOW_REMOTE = false;

/** 关于页的 GitHub、更新日志按钮不展示；许可证与上游项目说明保留（Apache-2.0 归属要求）。 */
export const TDPRINT_SHOW_REPOSITORY_LINKS = false;

/**
 * 内置网站白名单：插件后端始终放行（crates/runtime/src/tdprint_auth.rs 的 BUILTIN_ORIGINS，两处同步维护），
 * 界面上隐藏，避免仓库人员误删。
 */
export const TDPRINT_BUILTIN_ORIGINS: readonly string[] = [
  'https://oms.tongdacdsc.com',
  'https://pt.kuasuda.com',
];

/** 内置 IP 白名单（本机回环地址，上游已固定且不可删除），界面上隐藏。 */
export const TDPRINT_BUILTIN_IPS: readonly string[] = ['127.0.0.1'];

/** 网站白名单列表去掉内置项。 */
export function visibleAllowedOrigins(origins: readonly string[]): string[] {
  return origins.filter((origin) => !TDPRINT_BUILTIN_ORIGINS.includes(origin));
}

/** IP 白名单列表去掉内置项。 */
export function visibleAllowedIps(entries: readonly string[]): string[] {
  return entries.filter((entry) => !TDPRINT_BUILTIN_IPS.includes(entry));
}

/** 状态检测分组：远程任务未开放时不展示其分组。 */
export function visibleDoctorGroups<T extends { key: string }>(groups: T[]): T[] {
  return TDPRINT_SHOW_REMOTE ? groups : groups.filter((group) => group.key !== 'remote');
}

/** 诊断日志默认文件名：TdPrint-诊断日志-20261125-153000.txt（本机时间）。 */
export function diagnosticsFileName(now: Date = new Date()): string {
  const pad = (value: number) => String(value).padStart(2, '0');
  const date = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}`;
  const time = `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `TdPrint-诊断日志-${date}-${time}.txt`;
}

/** 把诊断日志（运行环境、最近任务、应用日志）写到指定文件，返回字节数（见 src-tauri/src/tdprint.rs）。 */
export function exportDiagnostics(path: string): Promise<number> {
  return invoke<number>('tdprint_export_diagnostics', { path });
}
