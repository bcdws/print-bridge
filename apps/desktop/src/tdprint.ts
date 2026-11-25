/**
 * TdPrint 发行版界面定制。集中在本文件，App.vue 等上游文件只保留少量引用，便于合并上游更新。
 */

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
