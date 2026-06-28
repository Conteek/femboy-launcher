export type Locale = 'ru' | 'en';

export interface Strings {
  // VersionDropdown
  loadingVersions: string;
  selectVersion: string;

  // AccountDropdown
  noAccounts: string;
  manageAccounts: string;

  // AccountsPage
  accountsTitle: string;
  close: string;
  noAccountsAdded: string;
  removeAccount: string;
  cancel: string;
  enterNickname: string;
  addOffline: string;
  loginElyBy: string;
  loginMicrosoft: string;
  addOnline: string;

  // UpdateModal
  updatingLauncher: string;

  // Action buttons
  loading: string;
  installing: string;
  launching: string;
  play: string;
  install: string;
  selectAccount: string;

  // Progress / Status banners
  minecraftLaunching: string;
  updateAvailable: string;

  // Error messages (dynamic parts use functions below)
  errorLoadVersionsOffline: string;
  errorLoadVersionsPrefix: string;
  errorInstallPrefix: string;
  errorLaunchPrefix: string;
  errorElyByPrefix: string;
  errorMicrosoftPrefix: string;

  // Progress messages
  startingInstall: string;
  waitingBrowserAuth: string;

  // RPC
  rpcInLauncher: string;
  rpcBrowsingMain: string;
  rpcInGame: string;
  rpcPlayingOn: (version: string) => string;

  // Titlebar tooltips
  gameFolder: string;
  settings: string;

  // SettingsPage
  settingsTitle: string;
  sectionGame: string;
  sectionAppearance: string;
  ram: string;
  mib: string;
  versionList: string;
  installedOnly: string;
  launcherVersion: string;
  jreRecommended: string;
  language: string;
  accentColor: string;
  customColor: string;
  theme: string;

  // Misc tabs (unused but kept for completeness)
  selectGameVersion: string;
  selectGameVersionDesc: string;
  friends: string;
  friendsDesc: string;
  launcherSettings: string;
  launcherSettingsDesc: string;

  // Modpacks
  modpacksTitle: string;
  addModpack: string;
  createModpack: string;
  editModpack: string;
  importModpack: string;
  importModpackSoon: string;
  noModpacks: string;
  deleteModpackConfirm: (name: string) => string;
  errorDeletingModpack: string;
  openFolder: string;
  edit: string;
  delete: string;
  clickOrDragImage: string;
  blankMode: string;
  snapshotMode: string;
  foldersToCopy: string;
  nameLabel: string;
  namePlaceholder: string;
  descriptionLabel: string;
  descriptionPlaceholder: string;
  authorLabel: string;
  versionLabel: string;
  creating: string;
  create: string;
  saving: string;
  save: string;
  nameRequired: string;
  versionRequired: string;
  export: string;
  modpackExported: string;
  modpackImported: string;
  unknownAuthor: string;
  modlist: string;
  modsCount: (count: number) => string;
  loadingMods: string;
  noModsFound: string;

  // Mod Manager
  modManager: string;
  modManagerTitle: string;
  modManagerNoMods: string;
  modManagerLoading: string;
  modEnabled: string;
  modDisabled: string;

  // Statistics
  statistics: string;
  totalPlaytime: string;
  playtimeTwoWeeks: string;
  period24h: string;
  period1w: string;
  period2w: string;
  periodAllTime: string;
  hoursShort: string;
}

const ru: Strings = {
  loadingVersions: 'Загрузка версий...',
  selectVersion: 'Выберите версию',

  noAccounts: 'Нет аккаунтов',
  manageAccounts: 'Управление аккаунтами',

  accountsTitle: 'Управление аккаунтами',
  close: 'Закрыть',
  noAccountsAdded: 'Нет добавленных аккаунтов',
  removeAccount: 'Удалить',
  cancel: 'Отмена',
  enterNickname: 'Введите никнейм...',
  addOffline: 'Добавить оффлайн',
  loginElyBy: 'Войти через Ely.by',
  loginMicrosoft: 'Войти через Microsoft',
  addOnline: 'Добавить онлайн',

  updatingLauncher: 'Обновление лаунчера',

  loading: 'Загрузка...',
  installing: 'Установка...',
  launching: 'Запуск...',
  play: 'ИГРАТЬ',
  install: 'УСТАНОВИТЬ',
  selectAccount: 'Выберите аккаунт',

  minecraftLaunching: 'Minecraft запускается...',
  updateAvailable: 'Доступно обновление',

  errorLoadVersionsOffline: 'Не удалось загрузить список версий и нет скачанных версий.',
  errorLoadVersionsPrefix: 'Не удалось загрузить список версий: ',
  errorInstallPrefix: 'Ошибка установки: ',
  errorLaunchPrefix: 'Ошибка запуска: ',
  errorElyByPrefix: 'Ошибка авторизации Ely.by: ',
  errorMicrosoftPrefix: 'Ошибка авторизации Microsoft: ',

  startingInstall: 'Начинаем установку...',
  waitingBrowserAuth: 'Ожидание авторизации в браузере...',

  rpcInLauncher: 'В лаунчере',
  rpcBrowsingMain: 'Просматривает главную страницу',
  rpcInGame: 'В игре',
  rpcPlayingOn: (v) => `Играет на ${v}`,

  gameFolder: 'Папка игры',
  settings: 'Настройки',

  settingsTitle: 'Настройки',
  sectionGame: 'Игра',
  sectionAppearance: 'Внешний вид',
  ram: 'Оперативная память',
  mib: ' МиБ',
  versionList: 'Список версий',
  installedOnly: 'Только установленные',
  launcherVersion: 'Версия лаунчера',
  jreRecommended: 'Рекомендуемая',
  language: 'Язык',
  accentColor: 'Цвет акцента',
  customColor: 'Свой цвет',
  theme: 'Тема',

  selectGameVersion: 'Выберите версию игры',
  selectGameVersionDesc:
    'Выберите нужную версию Minecraft в настройках и нажмите ИГРАТЬ, чтобы начать своё приключение.',
  friends: 'Ваши друзья',
  friendsDesc: 'Общайтесь с друзьями и присоединяйтесь к их серверам в один клик.',
  launcherSettings: 'Настройки лаунчера',
  launcherSettingsDesc:
    'Настраивайте выделение ОЗУ, аргументы Java и другие параметры запуска.',

  modpacksTitle: 'Модпаки',
  addModpack: 'Добавить модпак',
  createModpack: 'Создать модпак',
  editModpack: 'Редактировать модпак',
  importModpack: 'Загрузить модпака',
  importModpackSoon: 'Импорт модпаков — скоро!',
  noModpacks: 'Пока нет модпаков. Нажмите "Добавить модпак", чтобы создать.',
  deleteModpackConfirm: (name) => `Удалить модпак "${name}"? Это удалит все файлы в его папке.`,
  errorDeletingModpack: 'Ошибка удаления модпака: ',
  openFolder: 'Открыть папку',
  edit: 'Редактировать',
  delete: 'Удалить',
  clickOrDragImage: 'Кликните или перетащите',
  blankMode: 'Пустая',
  snapshotMode: 'Снапшот',
  foldersToCopy: 'Папки для копирования',
  nameLabel: 'Название',
  namePlaceholder: 'Мой модпак',
  descriptionLabel: 'Описание',
  descriptionPlaceholder: 'Необязательно...',
  authorLabel: 'Автор',
  versionLabel: 'Версия',
  creating: 'Создание...',
  create: 'Создать',
  saving: 'Сохранение...',
  save: 'Сохранить',
  nameRequired: 'Название обязательно',
  versionRequired: 'Версия обязательна',
  export: 'Экспорт',
  modpackExported: 'Модпак успешно экспортирован!',
  modpackImported: 'Модпак успешно загружен!',
  unknownAuthor: 'Неизвестный автор',
  modlist: 'Список модов',
  modsCount: (count) => `${count} модов`,
  loadingMods: 'Загрузка...',
  noModsFound: 'Моды не найдены',

  modManager: 'Диспетчер модов',
  modManagerTitle: 'Диспетчер модов',
  modManagerNoMods: 'В папке mods нет модов',
  modManagerLoading: 'Загрузка модов...',
  modEnabled: 'Включён',
  modDisabled: 'Выключен',

  statistics: 'Статистика',
  totalPlaytime: 'Всего времени в игре',
  playtimeTwoWeeks: 'За последние 2 недели',
  period24h: 'За 24 часа',
  period1w: 'За неделю',
  period2w: 'За 2 недели',
  periodAllTime: 'За все время',
  hoursShort: 'ч',
};

const en: Strings = {
  loadingVersions: 'Loading versions...',
  selectVersion: 'Select version',

  noAccounts: 'No accounts',
  manageAccounts: 'Manage accounts',

  accountsTitle: 'Manage Accounts',
  close: 'Close',
  noAccountsAdded: 'No accounts added',
  removeAccount: 'Remove',
  cancel: 'Cancel',
  enterNickname: 'Enter nickname...',
  addOffline: 'Add offline',
  loginElyBy: 'Sign in with Ely.by',
  loginMicrosoft: 'Sign in with Microsoft',
  addOnline: 'Add online',

  updatingLauncher: 'Updating launcher',

  loading: 'Loading...',
  installing: 'Installing...',
  launching: 'Launching...',
  play: 'PLAY',
  install: 'INSTALL',
  selectAccount: 'Select account',

  minecraftLaunching: 'Minecraft is launching...',
  updateAvailable: 'Update available',

  errorLoadVersionsOffline:
    'Could not load version list and no downloaded versions found.',
  errorLoadVersionsPrefix: 'Could not load version list: ',
  errorInstallPrefix: 'Installation error: ',
  errorLaunchPrefix: 'Launch error: ',
  errorElyByPrefix: 'Ely.by auth error: ',
  errorMicrosoftPrefix: 'Microsoft auth error: ',

  startingInstall: 'Starting installation...',
  waitingBrowserAuth: 'Waiting for browser auth...',

  rpcInLauncher: 'In launcher',
  rpcBrowsingMain: 'Browsing main page',
  rpcInGame: 'In game',
  rpcPlayingOn: (v) => `Playing ${v}`,

  gameFolder: 'Game folder',
  settings: 'Settings',

  settingsTitle: 'Settings',
  sectionGame: 'Game',
  sectionAppearance: 'Appearance',
  ram: 'RAM',
  mib: ' MiB',
  versionList: 'Version list',
  installedOnly: 'Installed only',
  launcherVersion: 'Launcher version',
  jreRecommended: 'Recommended',
  language: 'Language',
  accentColor: 'Accent Color',
  customColor: 'Custom color',
  theme: 'Theme',

  selectGameVersion: 'Select game version',
  selectGameVersionDesc:
    'Choose the Minecraft version in settings and press PLAY to start your adventure.',
  friends: 'Your friends',
  friendsDesc: 'Chat with friends and join their servers in one click.',
  launcherSettings: 'Launcher settings',
  launcherSettingsDesc: 'Configure RAM, Java arguments and other launch parameters.',

  modpacksTitle: 'Modpacks',
  addModpack: 'Add modpack',
  createModpack: 'Create modpack',
  editModpack: 'Edit modpack',
  importModpack: 'Import modpack',
  importModpackSoon: 'Import modpack — coming soon!',
  noModpacks: 'No modpacks yet. Click "Add modpack" to create one.',
  deleteModpackConfirm: (name) => `Delete modpack "${name}"? This will delete all files in its folder.`,
  errorDeletingModpack: 'Error deleting modpack: ',
  openFolder: 'Open folder',
  edit: 'Edit',
  delete: 'Delete',
  clickOrDragImage: 'Click or drag image',
  blankMode: 'Blank',
  snapshotMode: 'Snapshot',
  foldersToCopy: 'Folders to copy',
  nameLabel: 'Name',
  namePlaceholder: 'My Modpack',
  descriptionLabel: 'Description',
  descriptionPlaceholder: 'Optional...',
  authorLabel: 'Author',
  versionLabel: 'Version',
  creating: 'Creating...',
  create: 'Create',
  saving: 'Saving...',
  save: 'Save',
  nameRequired: 'Name is required',
  versionRequired: 'Version is required',
  export: 'Export',
  modpackExported: 'Modpack exported successfully!',
  modpackImported: 'Modpack imported successfully!',
  unknownAuthor: 'Unknown author',
  modlist: 'Modlist',
  modsCount: (count) => `${count} mods`,
  loadingMods: 'Loading...',
  noModsFound: 'No mods found',

  modManager: 'Mod Manager',
  modManagerTitle: 'Mod Manager',
  modManagerNoMods: 'No mods in the mods folder',
  modManagerLoading: 'Loading mods...',
  modEnabled: 'Enabled',
  modDisabled: 'Disabled',

  statistics: 'Statistics',
  totalPlaytime: 'Total playtime',
  playtimeTwoWeeks: 'Playtime (last 2 weeks)',
  period24h: 'Last 24 hours',
  period1w: 'Last week',
  period2w: 'Last 2 weeks',
  periodAllTime: 'All time',
  hoursShort: 'h',
};

const locales: Record<Locale, Strings> = { ru, en };

function detectLocale(): Locale {
  const lang = navigator.language?.toLowerCase() ?? 'ru';
  if (lang.startsWith('ru')) return 'ru';
  return 'en';
}

let _currentLocale: Locale = detectLocale();

export function getLocale(): Locale {
  return _currentLocale;
}

export function setLocale(locale: Locale): void {
  _currentLocale = locale;
}

export function t(): Strings {
  return locales[_currentLocale];
}
