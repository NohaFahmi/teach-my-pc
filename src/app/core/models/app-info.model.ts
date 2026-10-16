export interface AppInfo {
  name: string;
  version: string;
}

export type AppInfoState =
  | { status: 'loading' }
  | { status: 'available'; info: AppInfo }
  | { status: 'unavailable'; message: string };
