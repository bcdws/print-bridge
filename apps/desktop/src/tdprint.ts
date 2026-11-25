/**
 * TdPrint 发行版界面定制。集中在本文件，App.vue 等上游文件只保留少量引用，便于合并上游更新。
 */
import { invoke } from '@tauri-apps/api/core';

/** 远程任务 Tab（以及状态检测里的远程任务分组）暂不开放。 */
export const TDPRINT_SHOW_REMOTE = false;

/** 关于页不跳转 GitHub：隐藏 GitHub、更新日志、上游项目、许可证按钮；许可声明文字保留，
 * 许可证全文随安装目录提供（tauri.windows.conf.json 的 LICENSE.txt、THIRD_PARTY_NOTICES.md）。 */
export const TDPRINT_SHOW_REPOSITORY_LINKS = false;

/** 不附带 print-bridge 命令行工具（发版不再构建 CLI），设置页隐藏命令行工具一栏。 */
export const TDPRINT_SHOW_CLI = false;

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

/** 状态检测隐藏的检查项：Office 转换器（Office 打印已禁用，见 tdprint_policy.rs）、未开放的远程任务。 */
function isHiddenDoctorCheck(code: string): boolean {
  return code.startsWith('office.') || (!TDPRINT_SHOW_REMOTE && code.startsWith('remote.'));
}

/** 去掉隐藏的检查项并重算汇总，「需处理」只看剩下的检查项。 */
export function tdprintDoctorReport<
  T extends {
    checks: { code: string; status: string }[];
    summary: { pass: number; warn: number; fail: number };
  },
>(report: T): T {
  const checks = report.checks.filter((check) => !isHiddenDoctorCheck(check.code));
  const count = (status: string) => checks.filter((check) => check.status === status).length;
  return {
    ...report,
    checks,
    summary: { ...report.summary, pass: count('PASS'), warn: count('WARN'), fail: count('FAIL') },
  };
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
