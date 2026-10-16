import { Routes } from '@angular/router';
import { DashboardPagePage } from './features/dashboard/dashboard-page.page';
import { SettingsPagePage } from './features/settings/settings-page.page';
import { EmptyFeaturePage } from './shared/components/empty-feature';
import { RuleCreationPage } from './features/rules/rule-creation.page';

export const routes: Routes = [
  { path: '', component: DashboardPagePage, title: 'Dashboard | TeachMyPC' },
  {
    path: 'rules',
    component: RuleCreationPage,
    title: 'My Rules | TeachMyPC',
    data: { title: 'My Rules', description: 'Describe a file task and review its safe preview.' },
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
