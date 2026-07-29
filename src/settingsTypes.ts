export type JrePreference = 'recommended' | '8' | '17' | '21' | '25';
export type VersionTypeFilter = 'vanilla' | 'forge' | 'forgeOptifine' | 'fabric' | 'neoforge' | 'snapshots';

export interface VersionFilterSettings {
  types: Record<VersionTypeFilter, boolean>;
}

export const DEFAULT_VERSION_FILTERS: VersionFilterSettings = {
  types: {
    vanilla: true,
    forge: true,
    forgeOptifine: true,
    fabric: true,
    neoforge: true,
    snapshots: false,
  },
};
