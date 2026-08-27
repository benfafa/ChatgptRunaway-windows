import { useState } from "react";

export type Language = "zh-CN" | "en-US";

const STORAGE_KEY = "codex_runway_lang";

export const translations = {
  "zh-CN": {
    appTitle: "Codex Runway",
    activeBadge: "当前活跃",
    refreshTitle: "刷新",
    refreshing: "正在刷新…",
    localOnlyFooter: "纯本地运行 • 不上传任何 Token 或隐私凭据",
    settings: "设置",
    quit: "退出",
    
    // Accounts Card & Subscription
    accountsTitle: "账号管理",
    noAccounts: "暂无 Codex 账号。",
    addAccountBtn: "添加账号",
    reauthRequired: "需重新授权",
    activateBtn: "切换使用",
    removeBtn: "从列表中移除",
    subscriptionValidUntil: "订阅有效期至",
    
    // Session Repair
    sessionRepairTitle: "会话修复",
    sessionRepairStatus: "{missing} 缺失, {orphan} 孤立, {duplicate} 重复",
    sessionRepairAction: "修复索引",
    sessionRepairing: "正在修复索引…",
    sessionRepairSuccess: "已成功重建并修复 {count} 条会话索引！",
    
    // Recent Sessions
    recentSessionsTitle: "最近会话",
    recentlyActive: "最近活跃",
    
    // Quota Card & Estimations
    quotaTitle: "用量配额",
    quotaUsed: "已使用",
    quotaRemaining: "剩余可用",
    resetsIn: "重置倒计时",
    weeklyQuota: "周配额 (7-Day)",
    creditsBalance: "预付余额",
    runwayEstimateTitle: "额度推算",
    runwayBurnRate: "当前消耗速度",
    runwayRunout: "预计额度耗尽时间",
    runwaySafe: "配额充裕，可放心使用",
    runwayCritical: "配额消耗过快，请注意调配",
    
    // Reset Credits Card
    resetCreditsTitle: "重置次数与信用点",
    availableCreditsTag: "次可用",
    creditStatusAvailable: "可用",
    creditStatusUsed: "已使用",
    creditExpiresIn: "有效期剩余",
    todayResetStatus: "今日重置状态",
    todayResetReady: "今日可刷新重置",
    
    // Visualization & Usage
    usageTitle: "本地 Token 统计与图表",
    sessionsTurnsCount: "个会话 · {turns} 个交互轮次",
    totalTokens: "总 Token 消耗",
    inputTokensCached: "输入 (缓存率 {rate}%)",
    outputTokens: "输出 Token",
    chartHeatmapTab: "📅 活动热力图",
    chartTrendTab: "📈 消耗趋势图",
    chartBarTab: "📊 模型分布图",
    last30Days: "最近 30 天活动",
    noSessionsFound: "尚未发现本地 Codex 会话记录。",
    
    // Sessions Management & Backup/Restore
    sessionManagerTitle: "会话历史与管理",
    showSessionsList: "查看会话列表 ({count})",
    hideSessionsList: "收起会话列表",
    searchSessionsPlaceholder: "搜索工作目录、会话标题或模型…",
    backupBtn: "📦 备份会话",
    restoreBtn: "📥 还原会话",
    backupSuccess: "成功备份 {count} 个文件至 {path}",
    restoreSuccess: "成功还原 {count} 个会话文件",
    openFolder: "打开所在目录",
    
    // API Equivalent Cost
    apiCostTitle: "API 等价成本估算",
    estimatedTotal: "估算等价总额",
    pricedTurns: "{priced} 轮已定价 · {unknown} 轮未定价",
    unknownModel: "未知模型",
    showMore: "展开更多 ({count})",
    showFewer: "收起",
    pricingVersion: "计费规则版本",
    
    // Dialogs
    dialogAddTitle: "添加 Codex 账号",
    dialogAddSubtitle: "通过 ChatGPT 官方授权登录，或粘贴已有 ~/.codex/auth.json 文件内容。",
    dialogOAuthTab: "ChatGPT 网页授权登录",
    dialogPasteTab: "粘贴 auth.json",
    dialogOAuthPrompt: "点击下方按钮将在系统默认浏览器中打开 ChatGPT 授权页。授权完成后，本窗口将自动同步最新账号。",
    dialogOAuthStartBtn: "打开 ChatGPT 授权页",
    dialogOAuthOpening: "正在准备…",
    dialogOAuthWaiting: "等待网页授权完成…",
    dialogOAuthDone: "授权完成",
    dialogOAuthRetry: "重试授权",
    dialogLabelField: "账号备注/标签 (选填)",
    dialogLabelPlaceholder: "例如: 个人账号 / 工作 / Team",
    dialogAuthPlaceholder: '{\n  "auth_mode": "chatgpt",\n  "tokens": { ... }\n}',
    dialogCancel: "取消",
    dialogSubmitAdd: "添加并保存",
    
    // Language Switcher
    langToggle: "EN",
  },
  "en-US": {
    appTitle: "Codex Runway",
    activeBadge: "Active",
    refreshTitle: "Refresh",
    refreshing: "Refreshing…",
    localOnlyFooter: "Local-only • no tokens or secrets uploaded",
    settings: "Settings",
    quit: "Quit",
    
    // Accounts Card & Subscription
    accountsTitle: "Accounts",
    noAccounts: "No Codex accounts in the library.",
    addAccountBtn: "Add account",
    reauthRequired: "re-auth required",
    activateBtn: "Activate",
    removeBtn: "Remove",
    subscriptionValidUntil: "Subscription valid until",
    
    // Session Repair
    sessionRepairTitle: "Session Repair",
    sessionRepairStatus: "{missing} missing, {orphan} orphan, {duplicate} duplicate",
    sessionRepairAction: "Repair Index",
    sessionRepairing: "Repairing index…",
    sessionRepairSuccess: "Successfully rebuilt and repaired {count} session index items!",
    
    // Recent Sessions
    recentSessionsTitle: "Recent Sessions",
    recentlyActive: "Recent",
    
    // Quota Card & Estimations
    quotaTitle: "Quota",
    quotaUsed: "Used",
    quotaRemaining: "Remaining",
    resetsIn: "Resets in",
    weeklyQuota: "Weekly (7-Day)",
    creditsBalance: "Credits balance",
    runwayEstimateTitle: "Quota Runway",
    runwayBurnRate: "Burn Rate",
    runwayRunout: "Estimated Depletion",
    runwaySafe: "Quota is healthy",
    runwayCritical: "Heavy burn rate, monitor usage",
    
    // Reset Credits Card
    resetCreditsTitle: "Reset Credits",
    availableCreditsTag: "available",
    creditStatusAvailable: "available",
    creditStatusUsed: "used",
    creditExpiresIn: "expires in",
    todayResetStatus: "Reset Status",
    todayResetReady: "Ready to reset today",
    
    // Visualization & Usage
    usageTitle: "Token Usage & Charts",
    sessionsTurnsCount: "sessions · {turns} turns",
    totalTokens: "Total Tokens",
    inputTokensCached: "Input (Cached {rate}%)",
    outputTokens: "Output",
    chartHeatmapTab: "📅 Activity Heatmap",
    chartTrendTab: "📈 Usage Trend",
    chartBarTab: "📊 Model Distribution",
    last30Days: "Last 30 days activity",
    noSessionsFound: "No Codex session logs found yet.",
    
    // Sessions Management & Backup/Restore
    sessionManagerTitle: "Session Management",
    showSessionsList: "Show Sessions ({count})",
    hideSessionsList: "Hide Sessions",
    searchSessionsPlaceholder: "Search by title, directory, or model…",
    backupBtn: "📦 Backup Sessions",
    restoreBtn: "📥 Restore Sessions",
    backupSuccess: "Successfully backed up {count} files to {path}",
    restoreSuccess: "Successfully restored {count} session files",
    openFolder: "Open Folder",
    
    // API Equivalent Cost
    apiCostTitle: "API-Equivalent Cost",
    estimatedTotal: "Estimated Total",
    pricedTurns: "{priced} priced · {unknown} unknown",
    unknownModel: "unknown model",
    showMore: "Show all ({count})",
    showFewer: "Show fewer",
    pricingVersion: "Pricing version",
    
    // Dialogs
    dialogAddTitle: "Add Codex Account",
    dialogAddSubtitle: "Sign in with ChatGPT, or paste an existing ~/.codex/auth.json file.",
    dialogOAuthTab: "Sign in with ChatGPT",
    dialogPasteTab: "Paste auth.json",
    dialogOAuthPrompt: "Click below to open ChatGPT sign-in in your browser. After you finish, this window will refresh with the new account ready to use.",
    dialogOAuthStartBtn: "Open ChatGPT sign-in",
    dialogOAuthOpening: "Preparing…",
    dialogOAuthWaiting: "Waiting for sign-in…",
    dialogOAuthDone: "Done",
    dialogOAuthRetry: "Retry",
    dialogLabelField: "Label (optional)",
    dialogLabelPlaceholder: "personal · work · plus",
    dialogAuthPlaceholder: '{\n  "auth_mode": "chatgpt",\n  "tokens": { ... }\n}',
    dialogCancel: "Cancel",
    dialogSubmitAdd: "Add",
    
    // Language Switcher
    langToggle: "中",
  }
};

export function useLanguage() {
  const [lang, setLang] = useState<Language>(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en-US" || saved === "zh-CN") {
      return saved;
    }
    // Default to Simplified Chinese
    return "zh-CN";
  });

  const toggle = () => {
    setLang((prev) => {
      const next = prev === "zh-CN" ? "en-US" : "zh-CN";
      localStorage.setItem(STORAGE_KEY, next);
      return next;
    });
  };

  const t = translations[lang];

  return { lang, toggle, t };
}
