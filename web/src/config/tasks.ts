// MAA 任务配置 Schema
//
// 这是整个前端动态表单的核心：所有 MAA 任务类型和参数都定义在这里。
// 前端根据此 schema 自动渲染表单，无需修改组件代码。
//
// 【防御性设计】
// - 未来 MAA 增加新任务类型 → 在 tasks 数组加一个条目即可
// - 未来 MAA 增加/修改参数 → 修改对应 task 的 fields 即可
// - 未知字段透传：前端会保留所有字段，MAA 新参数自动兼容

export interface FieldOption {
  label: string
  value: string | number
}

export interface TaskField {
  name: string
  label: string
  type: 'number' | 'string' | 'boolean' | 'select' | 'multi-select' | 'textarea'
  default?: unknown
  required?: boolean
  options?: FieldOption[]
  placeholder?: string
  description?: string
}

export interface TaskType {
  type: string
  label: string
  icon: string
  description: string
  defaultParams: Record<string, unknown>
  fields: TaskField[]
}

// 客户端类型（多个任务共用）
const CLIENT_TYPES: FieldOption[] = [
  { label: '官服', value: 'Official' },
  { label: 'B 服', value: 'Bilibili' },
  { label: 'txwy', value: 'txwy' },
  { label: '国际服', value: 'YoStarEN' },
  { label: '日服', value: 'YoStarJP' },
  { label: '韩服', value: 'YoStarKR' },
]

const SERVER_TYPES: FieldOption[] = [
  { label: 'CN 国服', value: 'CN' },
  { label: 'US 国际服', value: 'US' },
  { label: 'JP 日服', value: 'JP' },
  { label: 'KR 韩服', value: 'KR' },
]

// 设施列表（基建）
const FACILITIES: FieldOption[] = [
  { label: '制造站', value: 'Mfg' },
  { label: '贸易站', value: 'Trade' },
  { label: '发电站', value: 'Power' },
  { label: '控制中枢', value: 'Control' },
  { label: '会客室', value: 'Reception' },
  { label: '办公室', value: 'Office' },
  { label: '宿舍', value: 'Dorm' },
  { label: '加工站', value: 'Processing' },
  { label: '训练室', value: 'Training' },
]

// 无人机用途
const DRONES: FieldOption[] = [
  { label: '不使用', value: '_NotUse' },
  { label: '龙门币', value: 'Money' },
  { label: '合成玉', value: 'SyntheticJade' },
  { label: '作战记录', value: 'CombatRecord' },
  { label: '赤金', value: 'PureGold' },
  { label: '源石碎片', value: 'OriginStone' },
  { label: '芯片', value: 'Chip' },
]

// 肉鸽主题
const ROGUELIKE_THEMES: FieldOption[] = [
  { label: '傀影与猩红血钻', value: 'Phantom' },
  { label: '水月与深蓝之树', value: 'Mizuki' },
  { label: '探索者的银凇止境', value: 'Sami' },
  { label: '萨卡兹的无终奇语', value: 'Sarkaz' },
  { label: '岁的界园志异', value: 'JieGarden' },
]

// 肉鸽模式
const ROGUELIKE_MODES: FieldOption[] = [
  { label: '0 - 刷分/奖励', value: 0 },
  { label: '1 - 刷源石锭', value: 1 },
  { label: '4 - 凹开局', value: 4 },
  { label: '5 - 刷坍缩范式', value: 5 },
  { label: '6 - 月度小队', value: 6 },
  { label: '7 - 深入调查', value: 7 },
]

// 生息演算主题
const RECLAMATION_THEMES: FieldOption[] = [
  { label: '沙中之火', value: 'Fire' },
  { label: '沙洲遗闻', value: 'Tales' },
  { label: '重启锚点', value: 'RelaunchAnchor' },
]

// 全部任务定义
export const TASK_TYPES: TaskType[] = [
  {
    type: 'StartUp',
    label: '开始唤醒',
    icon: '🚀',
    description: '启动客户端并进入游戏',
    defaultParams: {
      enable: true,
      client_type: 'Official',
      start_game_enabled: false,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'client_type', label: '客户端版本', type: 'select', options: CLIENT_TYPES, default: 'Official' },
      { name: 'start_game_enabled', label: '自动启动客户端', type: 'boolean', default: false },
      { name: 'account_name', label: '切换账号', type: 'string', placeholder: '登录名片段' },
    ],
  },
  {
    type: 'Fight',
    label: '理智作战',
    icon: '⚔️',
    description: '刷理智，指定关卡或自动识别',
    defaultParams: {
      enable: true,
      stage: '',
      medicine: 0,
      stone: 0,
      times: 1,
      series: 0,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'stage', label: '关卡名', type: 'string', placeholder: '如 1-7 / CE-6 / Annihilation' },
      { name: 'medicine', label: '最大理智药数量', type: 'number', default: 0 },
      { name: 'medicine_expire_days', label: '过期理智药天数', type: 'number', default: 0 },
      { name: 'stone', label: '吃石头数量', type: 'number', default: 0 },
      { name: 'times', label: '战斗次数', type: 'number', default: 1 },
      { name: 'series', label: '连战次数', type: 'number', default: 0, description: '-1禁用，0自动，1-6指定' },
      { name: 'report_to_penguin', label: '汇报企鹅数据', type: 'boolean', default: false },
      { name: 'report_to_yituliu', label: '汇报一图流', type: 'boolean', default: false },
      { name: 'server', label: '服务器', type: 'select', options: SERVER_TYPES, default: 'CN' },
      { name: 'client_type', label: '客户端版本', type: 'select', options: CLIENT_TYPES },
      { name: 'DrGrandet', label: '省理智碎石模式', type: 'boolean', default: false },
    ],
  },
  {
    type: 'Recruit',
    label: '公开招募',
    icon: '🎯',
    description: '自动公招，支持 Tag 计算',
    defaultParams: {
      enable: true,
      refresh: false,
      select: [4, 5],
      confirm: [4, 3],
      times: 4,
      expedite: false,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'refresh', label: '刷新三星 Tags', type: 'boolean', default: false },
      { name: 'select', label: '点击标签的 Tag 等级', type: 'multi-select', default: [4, 5] },
      { name: 'confirm', label: '确认招募的 Tag 等级', type: 'multi-select', default: [4, 3] },
      { name: 'times', label: '招募次数', type: 'number', default: 4 },
      { name: 'set_time', label: '设置招募时限', type: 'boolean', default: true },
      { name: 'expedite', label: '使用加急许可', type: 'boolean', default: false },
      { name: 'expedite_times', label: '加急次数', type: 'number', default: 0 },
      { name: 'preserve_tags', label: '保留跳过 Tags', type: 'multi-select', default: [], placeholder: '如 支援机械' },
      { name: 'report_to_penguin', label: '汇报企鹅数据', type: 'boolean', default: false },
      { name: 'report_to_yituliu', label: '汇报一图流', type: 'boolean', default: false },
      { name: 'server', label: '服务器', type: 'select', options: SERVER_TYPES, default: 'CN' },
    ],
  },
  {
    type: 'Infrast',
    label: '基建换班',
    icon: '🏭',
    description: '自动换班，支持自定义方案',
    defaultParams: {
      enable: true,
      mode: 0,
      facility: ['Mfg', 'Trade'],
      drones: '_NotUse',
      threshold: 0.3,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      {
        name: 'mode', label: '工作模式', type: 'select',
        options: [
          { label: '0 - 默认换班', value: 0 },
          { label: '10000 - 自定义换班', value: 10000 },
          { label: '20000 - 一键轮换', value: 20000 },
        ],
        default: 0,
      },
      { name: 'facility', label: '换班设施', type: 'multi-select', default: ['Mfg', 'Trade'], options: FACILITIES },
      { name: 'drones', label: '无人机用途', type: 'select', options: DRONES, default: '_NotUse' },
      { name: 'threshold', label: '工作心情阈值', type: 'number', default: 0.3 },
      { name: 'replenish', label: '源石碎片自动补货', type: 'boolean', default: false },
      { name: 'dorm_notstationed_enabled', label: '启用宿舍未进驻', type: 'boolean', default: false },
      { name: 'dorm_trust_enabled', label: '宿舍填信赖未满干员', type: 'boolean', default: false },
      { name: 'reception_message_board', label: '领取会客室信息板', type: 'boolean', default: true },
      { name: 'reception_clue_exchange', label: '进行线索交流', type: 'boolean', default: true },
      { name: 'reception_send_clue', label: '赠送线索', type: 'boolean', default: true },
    ],
  },
  {
    type: 'Mall',
    label: '信用及商店',
    icon: '🛒',
    description: '领取信用，自动购物',
    defaultParams: {
      enable: true,
      visit_friends: true,
      shopping: true,
      buy_first: [],
      blacklist: [],
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'visit_friends', label: '访问好友基建', type: 'boolean', default: true },
      { name: 'shopping', label: '购物', type: 'boolean', default: true },
      { name: 'buy_first', label: '优先购买列表', type: 'textarea', placeholder: '如 招聘许可, 龙门币（逗号分隔）' },
      { name: 'blacklist', label: '购物黑名单', type: 'textarea', placeholder: '如 加急许可, 家具零件（逗号分隔）' },
      { name: 'force_shopping_if_credit_full', label: '信用溢出无视黑名单', type: 'boolean', default: false },
      { name: 'only_buy_discount', label: '只买折扣物品', type: 'boolean', default: false },
      { name: 'reserve_max_credit', label: '信用低于300停止购买', type: 'boolean', default: false },
      { name: 'credit_fight', label: '借助战打OF-1', type: 'boolean', default: false },
      { name: 'formation_index', label: '编队栏位编号', type: 'number', default: 0 },
    ],
  },
  {
    type: 'Award',
    label: '领取奖励',
    icon: '🎁',
    description: '领取各种日常奖励',
    defaultParams: {
      enable: true,
      award: true,
      mail: false,
      recruit: false,
      orundum: false,
      mining: false,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'award', label: '每日/每周任务奖励', type: 'boolean', default: true },
      { name: 'mail', label: '邮件奖励', type: 'boolean', default: false },
      { name: 'recruit', label: '限定池免费单抽', type: 'boolean', default: false },
      { name: 'orundum', label: '幸运墙合成玉', type: 'boolean', default: false },
      { name: 'mining', label: '限时开采合成玉', type: 'boolean', default: false },
      { name: 'specialaccess', label: '月卡奖励', type: 'boolean', default: false },
    ],
  },
  {
    type: 'Roguelike',
    label: '无限刷肉鸽',
    icon: '🏆',
    description: '自动刷集成战略',
    defaultParams: {
      enable: true,
      theme: 'Sami',
      mode: 0,
      squad: '指挥分队',
      roles: '取长补短',
      investment_enabled: true,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'theme', label: '主题', type: 'select', options: ROGUELIKE_THEMES, default: 'Sami' },
      { name: 'mode', label: '模式', type: 'select', options: ROGUELIKE_MODES, default: 0 },
      { name: 'squad', label: '开局分队', type: 'string', default: '指挥分队' },
      { name: 'roles', label: '开局职业组', type: 'string', default: '取长补短' },
      { name: 'core_char', label: '开局干员', type: 'string', placeholder: '干员中文名' },
      { name: 'use_support', label: '使用助战干员', type: 'boolean', default: false },
      { name: 'use_nonfriend_support', label: '允许非好友助战', type: 'boolean', default: false },
      { name: 'starts_count', label: '开始探索次数', type: 'number', default: 2147483647 },
      { name: 'difficulty', label: '难度等级', type: 'number', default: 0 },
      { name: 'stop_at_final_boss', label: '第5层险路恶敌停止', type: 'boolean', default: false },
      { name: 'stop_at_max_level', label: '肉鸽等级刷满停止', type: 'boolean', default: false },
      { name: 'investment_enabled', label: '投资源石锭', type: 'boolean', default: true },
      { name: 'investments_count', label: '投资次数', type: 'number', default: 2147483647 },
      { name: 'stop_when_investment_full', label: '投资满停止', type: 'boolean', default: false },
      { name: 'start_with_elite_two', label: '凹精二直升', type: 'boolean', default: false },
      { name: 'only_start_with_elite_two', label: '只凹精二直升', type: 'boolean', default: false },
    ],
  },
  {
    type: 'Copilot',
    label: '自动抄作业',
    icon: '📋',
    description: '按作业文件自动作战',
    defaultParams: {
      enable: true,
      filename: '',
      loop_times: 1,
      formation: false,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'filename', label: '作业文件路径', type: 'string', placeholder: 'copilot/1-7.json' },
      { name: 'loop_times', label: '循环次数', type: 'number', default: 1 },
      { name: 'use_sanity_potion', label: '使用理智药', type: 'boolean', default: false },
      { name: 'formation', label: '自动编队', type: 'boolean', default: false },
      { name: 'formation_index', label: '编队栏位编号', type: 'number', default: 0 },
      { name: 'add_trust', label: '自动填充信赖干员', type: 'boolean', default: false },
      { name: 'ignore_requirements', label: '忽视干员属性要求', type: 'boolean', default: false },
      { name: 'support_unit_usage', label: '助战干员模式', type: 'select', options: [
        { label: '0 - 不使用助战', value: 0 },
        { label: '1 - 缺失时寻找助战', value: 1 },
        { label: '2 - 缺失时使用指定助战', value: 2 },
        { label: '3 - 缺失时使用随机助战', value: 3 },
      ], default: 0 },
      { name: 'support_unit_name', label: '指定助战干员', type: 'string', placeholder: '如 艾雅法拉' },
    ],
  },
  {
    type: 'Reclamation',
    label: '生息演算',
    icon: '🌵',
    description: '自动生息演算',
    defaultParams: {
      enable: true,
      theme: 'Tales',
      mode: 0,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'theme', label: '主题', type: 'select', options: RECLAMATION_THEMES, default: 'Tales' },
      { name: 'mode', label: '模式', type: 'number', default: 0 },
      { name: 'tools_to_craft', label: '自动制造物品', type: 'string', placeholder: '如 荧光棒, 发电机（逗号分隔）' },
      { name: 'increment_mode', label: '点击类型', type: 'select', options: [
        { label: '0 - 连点', value: 0 },
        { label: '1 - 长按', value: 1 },
      ], default: 0 },
      { name: 'num_craft_batches', label: '最大制造轮数', type: 'number', default: 16 },
    ],
  },
  {
    type: 'Depot',
    label: '仓库识别',
    icon: '📦',
    description: '识别仓库物品',
    defaultParams: { enable: true },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
    ],
  },
  {
    type: 'OperBox',
    label: '干员识别',
    icon: '👤',
    description: '识别干员 Box',
    defaultParams: { enable: true },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
    ],
  },
  {
    type: 'Custom',
    label: '自定义任务',
    icon: '🧩',
    description: '按任务名执行自定义流程',
    defaultParams: {
      enable: true,
      task_names: ['StartUp', 'Infrast', 'Fight'],
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'task_names', label: '任务名列表', type: 'textarea', placeholder: '如 StartUp, Infrast, Fight（逗号分隔）' },
    ],
  },
  {
    type: 'SSSCopilot',
    label: '保全派驻',
    icon: '🛡️',
    description: '自动抄保全作业',
    defaultParams: {
      enable: true,
      filename: '',
      loop_times: 1,
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'filename', label: '作业文件路径', type: 'string', placeholder: 'sss/plan.json' },
      { name: 'loop_times', label: '循环次数', type: 'number', default: 1 },
    ],
  },
  {
    type: 'ParadoxCopilot',
    label: '悖论模拟',
    icon: '🧠',
    description: '自动抄悖论模拟作业',
    defaultParams: {
      enable: true,
      filename: '',
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'filename', label: '作业文件路径', type: 'string', placeholder: 'paradox/exusiai.json' },
    ],
  },
  {
    type: 'SingleStep',
    label: '单步任务',
    icon: '🎬',
    description: '单步作战操作',
    defaultParams: {
      enable: true,
      type: 'copilot',
      subtask: 'stage',
      details: { stage: '1-7' },
    },
    fields: [
      { name: 'enable', label: '启用任务', type: 'boolean', default: true },
      { name: 'subtask', label: '子任务类型', type: 'select', options: [
        { label: 'stage - 设置关卡', value: 'stage' },
        { label: 'start - 开始作战', value: 'start' },
        { label: 'action - 单步操作', value: 'action' },
      ], default: 'stage' },
      { name: 'details', label: '详细参数 (JSON)', type: 'textarea', placeholder: '如 {"stage":"1-7"}' },
    ],
  },
]

// 连接配置（模拟器类型）
export const CONNECT_CONFIGS = [
  { label: '通用', value: 'General' },
  { label: 'BlueStacks', value: 'BlueStacks' },
  { label: 'MuMu模拟器', value: 'MuMuEmulator' },
  { label: 'MuMu模拟器12', value: 'MuMuEmulator12' },
  { label: '雷电模拟器', value: 'LDPlayer' },
  { label: '夜神模拟器', value: 'NoxPlayer' },
  { label: '逍遥模拟器', value: 'XYAZ' },
  { label: 'Windows 子系统', value: 'WSA' },
  { label: 'Waydroid', value: 'Waydroid' },
]

// 默认任务面板（一键长草组合）
export const DEFAULT_TASKS = [
  { type: 'StartUp', label: '开始唤醒', params: { enable: true, client_type: 'Official' } },
  { type: 'Fight', label: '理智作战', params: { enable: true, stage: '', medicine: 0, times: 1 } },
  { type: 'Infrast', label: '基建换班', params: { enable: true, facility: ['Mfg', 'Trade'], drones: '_NotUse' } },
  { type: 'Recruit', label: '公开招募', params: { enable: true, times: 4 } },
  { type: 'Mall', label: '信用及商店', params: { enable: true } },
  { type: 'Award', label: '领取奖励', params: { enable: true } },
]

// 按类型查找任务定义
export function getTaskType(type: string): TaskType | undefined {
  return TASK_TYPES.find((t) => t.type === type)
}
