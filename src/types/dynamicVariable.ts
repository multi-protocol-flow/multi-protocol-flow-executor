/**
 * 动态变量系统类型定义与元数据
 * 与后端 src-tauri/crates/flow-engine-core/src/dynamic_variable.rs 保持严格对齐
 */

export type DynamicVariableCategory =
  | 'common'
  | 'datetime'
  | 'numeric'
  | 'network'
  | 'identity'
  | 'contact'
  | 'text'
  | 'location';

export interface DynamicVariableItem {
  /** 变量引用名称，如 '$guid' */
  name: string;
  /** 分类 */
  category: DynamicVariableCategory;
  /** 国际化多语言键路径 */
  descriptionKey: string;
  /** 静态标准示例 */
  example: string;
  /** 实时生成示例值 */
  generateSample: () => string;
}

const FIRST_NAMES = [
  'James', 'Mary', 'John', 'Patricia', 'Robert', 'Jennifer', 'Michael', 'Linda',
  'William', 'Elizabeth', 'David', 'Barbara', 'Richard', 'Susan', 'Joseph', 'Jessica',
  'Thomas', 'Sarah', 'Charles', 'Karen', 'Christopher', 'Nancy', 'Daniel', 'Lisa',
  'Matthew', 'Betty', 'Anthony', 'Margaret', 'Mark', 'Sandra', 'Alex', 'Sam',
  'Emma', 'Liam', 'Olivia', 'Noah', 'Sophia', 'Lucas', 'Mia', 'Ethan',
];

const LAST_NAMES = [
  'Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis',
  'Rodriguez', 'Martinez', 'Hernandez', 'Lopez', 'Gonzalez', 'Wilson', 'Anderson',
  'Thomas', 'Taylor', 'Moore', 'Jackson', 'Martin', 'Lee', 'Perez',
  'Thompson', 'White', 'Harris', 'Sanchez', 'Clark', 'Ramirez', 'Lewis', 'Robinson',
];

const DOMAINS = [
  'gmail.com', 'yahoo.com', 'hotmail.com', 'outlook.com', 'icloud.com',
  'proton.me', 'mail.com', 'fastmail.com',
];

const WORDS = [
  'lorem', 'ipsum', 'dolor', 'sit', 'amet', 'consectetur', 'adipiscing', 'elit',
  'sed', 'do', 'eiusmod', 'tempor', 'incididunt', 'ut', 'labore', 'et',
  'dolore', 'magna', 'aliqua', 'enim', 'ad', 'minim', 'veniam', 'quis',
];

const CITIES = [
  'New York', 'London', 'Tokyo', 'Paris', 'Berlin', 'Sydney', 'Beijing',
  'Singapore', 'Toronto', 'San Francisco', 'Seattle', 'Amsterdam', 'Seoul',
  'Dubai', 'Rome', 'Madrid', 'Chicago', 'Boston', 'Hong Kong', 'Zurich',
];

const COUNTRIES: [string, string][] = [
  ['United States', 'US'],
  ['China', 'CN'],
  ['United Kingdom', 'GB'],
  ['Germany', 'DE'],
  ['Japan', 'JP'],
  ['France', 'FR'],
  ['Australia', 'AU'],
  ['Canada', 'CA'],
  ['Singapore', 'SG'],
  ['Netherlands', 'NL'],
  ['South Korea', 'KR'],
  ['Switzerland', 'CH'],
  ['Spain', 'ES'],
  ['Italy', 'IT'],
  ['Brazil', 'BR'],
];

const CURRENCIES = ['USD', 'EUR', 'CNY', 'GBP', 'JPY', 'CAD', 'AUD', 'CHF', 'HKD', 'SGD'];

const COLORS = [
  'red', 'blue', 'green', 'yellow', 'purple', 'orange', 'pink', 'teal', 'cyan', 'indigo',
];

const COMPANIES = [
  'Acme Corp', 'Globex', 'Initech', 'Umbrella Corp', 'Soylent Corp',
  'Hooli', 'Pied Piper', 'Stark Industries', 'Wayne Enterprises', 'Cyberdyne Systems',
];

const JOB_TITLES = [
  'Software Engineer', 'Product Manager', 'DevOps Engineer', 'Data Scientist',
  'UI/UX Designer', 'QA Engineer', 'Engineering Manager', 'Technical Lead',
  'Security Analyst', 'System Architect',
];

function pickRandom<T>(arr: readonly T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

function randomInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function generateUuid(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * 所有受支持的动态变量定义完整列表
 */
export const DYNAMIC_VARIABLES: DynamicVariableItem[] = [
  // 通用标识
  {
    name: '$guid',
    category: 'common',
    descriptionKey: 'flow.dynamic_var_guid',
    example: 'c029fa8a-531e-4cb2-8356-65be7f61c6b1',
    generateSample: () => generateUuid(),
  },
  {
    name: '$uuid',
    category: 'common',
    descriptionKey: 'flow.dynamic_var_uuid',
    example: '8b7f833b-7419-4a0b-932d-327c134cf1f2',
    generateSample: () => generateUuid(),
  },
  {
    name: '$randomUUID',
    category: 'common',
    descriptionKey: 'flow.dynamic_var_randomUUID',
    example: '3f6c77bb-a3ee-4f30-80ea-37dc6b74e6ef',
    generateSample: () => generateUuid(),
  },

  // 日期时间
  {
    name: '$timestamp',
    category: 'datetime',
    descriptionKey: 'flow.dynamic_var_timestamp',
    example: '1719876543',
    generateSample: () => Math.floor(Date.now() / 1000).toString(),
  },
  {
    name: '$timestampMs',
    category: 'datetime',
    descriptionKey: 'flow.dynamic_var_timestampMs',
    example: '1719876543210',
    generateSample: () => Date.now().toString(),
  },
  {
    name: '$isoTimestamp',
    category: 'datetime',
    descriptionKey: 'flow.dynamic_var_isoTimestamp',
    example: '2024-07-02T12:34:56.789Z',
    generateSample: () => new Date().toISOString(),
  },
  {
    name: '$randomDatePast',
    category: 'datetime',
    descriptionKey: 'flow.dynamic_var_randomDatePast',
    example: '2023-11-14T08:12:30Z',
    generateSample: () => {
      const days = randomInt(1, 365);
      return new Date(Date.now() - days * 86400000).toISOString().replace(/\.\d{3}Z$/, 'Z');
    },
  },
  {
    name: '$randomDateFuture',
    category: 'datetime',
    descriptionKey: 'flow.dynamic_var_randomDateFuture',
    example: '2025-04-20T16:45:10Z',
    generateSample: () => {
      const days = randomInt(1, 365);
      return new Date(Date.now() + days * 86400000).toISOString().replace(/\.\d{3}Z$/, 'Z');
    },
  },
  {
    name: '$randomDateRecent',
    category: 'datetime',
    descriptionKey: 'flow.dynamic_var_randomDateRecent',
    example: '2024-06-25T10:20:30Z',
    generateSample: () => {
      const days = randomInt(1, 30);
      return new Date(Date.now() - days * 86400000).toISOString().replace(/\.\d{3}Z$/, 'Z');
    },
  },

  // 随机数值与格式
  {
    name: '$randomInt',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomInt',
    example: '428',
    generateSample: () => randomInt(0, 1000).toString(),
  },
  {
    name: '$randomPrice',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomPrice',
    example: '49.99',
    generateSample: () => (Math.random() * 998.99 + 1).toFixed(2),
  },
  {
    name: '$randomBoolean',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomBoolean',
    example: 'true',
    generateSample: () => (Math.random() > 0.5 ? 'true' : 'false'),
  },
  {
    name: '$randomAlphaNumeric',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomAlphaNumeric',
    example: 'k8Xb1A9q',
    generateSample: () => {
      const chars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
      let res = '';
      for (let i = 0; i < 8; i++) {
        res += chars.charAt(Math.floor(Math.random() * chars.length));
      }
      return res;
    },
  },
  {
    name: '$randomString',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomString',
    example: 'qazwsxed',
    generateSample: () => {
      let res = '';
      for (let i = 0; i < 8; i++) {
        res += String.fromCharCode(97 + randomInt(0, 25));
      }
      return res;
    },
  },
  {
    name: '$randomHexColor',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomHexColor',
    example: '#3b82f6',
    generateSample: () => '#' + randomInt(0, 0xffffff).toString(16).padStart(6, '0'),
  },
  {
    name: '$randomColor',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomColor',
    example: 'blue',
    generateSample: () => pickRandom(COLORS),
  },
  {
    name: '$randomSemver',
    category: 'numeric',
    descriptionKey: 'flow.dynamic_var_randomSemver',
    example: '1.4.12',
    generateSample: () => `${randomInt(0, 5)}.${randomInt(0, 20)}.${randomInt(0, 99)}`,
  },

  // 网络通信
  {
    name: '$randomIP',
    category: 'network',
    descriptionKey: 'flow.dynamic_var_randomIP',
    example: '192.168.1.101',
    generateSample: () => `${randomInt(1, 254)}.${randomInt(0, 255)}.${randomInt(0, 255)}.${randomInt(1, 254)}`,
  },
  {
    name: '$randomIPV6',
    category: 'network',
    descriptionKey: 'flow.dynamic_var_randomIPV6',
    example: '2001:0db8:85a3:0000:0000:8a2e:0370:7334',
    generateSample: () => {
      const parts = Array.from({ length: 8 }, () => randomInt(0, 0xffff).toString(16).padStart(4, '0'));
      return parts.join(':');
    },
  },
  {
    name: '$randomMACAddress',
    category: 'network',
    descriptionKey: 'flow.dynamic_var_randomMACAddress',
    example: '00:1a:2b:3c:4d:5e',
    generateSample: () => {
      const parts = Array.from({ length: 6 }, () => randomInt(0, 0xff).toString(16).padStart(2, '0'));
      return parts.join(':');
    },
  },
  {
    name: '$randomPort',
    category: 'network',
    descriptionKey: 'flow.dynamic_var_randomPort',
    example: '8080',
    generateSample: () => randomInt(1024, 65535).toString(),
  },
  {
    name: '$randomUrl',
    category: 'network',
    descriptionKey: 'flow.dynamic_var_randomUrl',
    example: 'https://gmail.com/lorem',
    generateSample: () => `https://${pickRandom(DOMAINS)}/${pickRandom(WORDS)}`,
  },
  {
    name: '$randomDomainName',
    category: 'network',
    descriptionKey: 'flow.dynamic_var_randomDomainName',
    example: 'example.com',
    generateSample: () => pickRandom(DOMAINS),
  },

  // 人员身份
  {
    name: '$randomFirstName',
    category: 'identity',
    descriptionKey: 'flow.dynamic_var_randomFirstName',
    example: 'James',
    generateSample: () => pickRandom(FIRST_NAMES),
  },
  {
    name: '$randomLastName',
    category: 'identity',
    descriptionKey: 'flow.dynamic_var_randomLastName',
    example: 'Smith',
    generateSample: () => pickRandom(LAST_NAMES),
  },
  {
    name: '$randomFullName',
    category: 'identity',
    descriptionKey: 'flow.dynamic_var_randomFullName',
    example: 'James Smith',
    generateSample: () => `${pickRandom(FIRST_NAMES)} ${pickRandom(LAST_NAMES)}`,
  },
  {
    name: '$randomUserName',
    category: 'identity',
    descriptionKey: 'flow.dynamic_var_randomUserName',
    example: 'james428',
    generateSample: () => `${pickRandom(FIRST_NAMES).toLowerCase()}${randomInt(10, 9999)}`,
  },
  {
    name: '$randomJobTitle',
    category: 'identity',
    descriptionKey: 'flow.dynamic_var_randomJobTitle',
    example: 'Software Engineer',
    generateSample: () => pickRandom(JOB_TITLES),
  },

  // 联系方式
  {
    name: '$randomEmail',
    category: 'contact',
    descriptionKey: 'flow.dynamic_var_randomEmail',
    example: 'alex.smith42@gmail.com',
    generateSample: () => {
      const first = pickRandom(FIRST_NAMES).toLowerCase();
      const last = pickRandom(LAST_NAMES).toLowerCase();
      const num = randomInt(1, 99);
      const domain = pickRandom(DOMAINS);
      return `${first}.${last}${num}@${domain}`;
    },
  },
  {
    name: '$randomExampleEmail',
    category: 'contact',
    descriptionKey: 'flow.dynamic_var_randomExampleEmail',
    example: 'alex.smith42@example.com',
    generateSample: () => {
      const first = pickRandom(FIRST_NAMES).toLowerCase();
      const last = pickRandom(LAST_NAMES).toLowerCase();
      const num = randomInt(1, 99);
      return `${first}.${last}${num}@example.com`;
    },
  },
  {
    name: '$randomPhoneNumber',
    category: 'contact',
    descriptionKey: 'flow.dynamic_var_randomPhoneNumber',
    example: '+1-555-123-4567',
    generateSample: () => `+1-${randomInt(200, 999)}-${randomInt(200, 999)}-${randomInt(1000, 9999)}`,
  },
  {
    name: '$randomPhone',
    category: 'contact',
    descriptionKey: 'flow.dynamic_var_randomPhone',
    example: '+1-555-123-4567',
    generateSample: () => `+1-${randomInt(200, 999)}-${randomInt(200, 999)}-${randomInt(1000, 9999)}`,
  },

  // 文本数据
  {
    name: '$randomWord',
    category: 'text',
    descriptionKey: 'flow.dynamic_var_randomWord',
    example: 'lorem',
    generateSample: () => pickRandom(WORDS),
  },
  {
    name: '$randomWords',
    category: 'text',
    descriptionKey: 'flow.dynamic_var_randomWords',
    example: 'lorem ipsum dolor',
    generateSample: () => {
      const count = randomInt(3, 5);
      const res: string[] = [];
      for (let i = 0; i < count; i++) {
        res.push(pickRandom(WORDS));
      }
      return res.join(' ');
    },
  },

  // 地理商业
  {
    name: '$randomCity',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomCity',
    example: 'New York',
    generateSample: () => pickRandom(CITIES),
  },
  {
    name: '$randomCountry',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomCountry',
    example: 'United States',
    generateSample: () => pickRandom(COUNTRIES)[0],
  },
  {
    name: '$randomCountryCode',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomCountryCode',
    example: 'US',
    generateSample: () => pickRandom(COUNTRIES)[1],
  },
  {
    name: '$randomLatitude',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomLatitude',
    example: '37.774929',
    generateSample: () => (Math.random() * 180 - 90).toFixed(6),
  },
  {
    name: '$randomLongitude',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomLongitude',
    example: '-122.419416',
    generateSample: () => (Math.random() * 360 - 180).toFixed(6),
  },
  {
    name: '$randomCurrencyCode',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomCurrencyCode',
    example: 'USD',
    generateSample: () => pickRandom(CURRENCIES),
  },
  {
    name: '$randomCompanyName',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomCompanyName',
    example: 'Acme Corp',
    generateSample: () => pickRandom(COMPANIES),
  },
  {
    name: '$randomStreetAddress',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomStreetAddress',
    example: '742 Main St',
    generateSample: () => `${randomInt(100, 9999)} ${pickRandom(LAST_NAMES)} St`,
  },
  {
    name: '$randomPostalCode',
    category: 'location',
    descriptionKey: 'flow.dynamic_var_randomPostalCode',
    example: '94105',
    generateSample: () => randomInt(10000, 99999).toString(),
  },
];

const DYNAMIC_VARIABLE_SET = new Set(DYNAMIC_VARIABLES.map((v) => v.name));

/**
 * 判断指定变量名称是否为动态变量
 * 支持传入 `$guid`、`{{$guid}}` 或带空白的格式
 */
export function isDynamicVariable(name: string): boolean {
  const normalized = name.trim().replace(/^\{\{\s*|\s*\}\}$/g, '').trim();
  return DYNAMIC_VARIABLE_SET.has(normalized);
}

/**
 * 按分类聚合所有动态变量
 */
export function getDynamicVariablesByCategory(): Record<DynamicVariableCategory, DynamicVariableItem[]> {
  const map: Record<DynamicVariableCategory, DynamicVariableItem[]> = {
    common: [],
    datetime: [],
    numeric: [],
    network: [],
    identity: [],
    contact: [],
    text: [],
    location: [],
  };

  for (const item of DYNAMIC_VARIABLES) {
    map[item.category].push(item);
  }

  return map;
}
