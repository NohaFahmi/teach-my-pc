import { Routes } from '@angular/router';
import { DashboardPagePage } from './features/dashboard/dashboard-page.page';
import { SettingsPagePage } from './features/settings/settings-page.page';
import { EmptyFeaturePage } from './shared/components/empty-feature';

export const routes: Routes = [
  { path: '', component: DashboardPagePage, title: 'Dashboard | TeachMyPC' },
  {
    path: 'rules',
    component: EmptyFeaturePage,
    title: 'My Rules | TeachMyPC',
    data: { title: 'My Rules', description: 'Review and manage the automations you create for your Mac.' },
  },
  {
    path: 'activity',
    component: EmptyFeaturePage,
    title: 'Activity | TeachMyPC',
    data: { title: 'Activity', description: 'A clear history of what your approved automations do.' },
  },
  { path: 'settings', component: SettingsPagePage, title: 'Settings | TeachMyPC' },
  { path: '**', redirectTo: '' },
];
