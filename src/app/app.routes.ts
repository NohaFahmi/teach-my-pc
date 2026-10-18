import { Routes } from '@angular/router';
import { DashboardPagePage } from './features/dashboard/dashboard-page.page';
import { SettingsPagePage } from './features/settings/settings-page.page';
import { RuleCreationPage } from './features/rules/rule-creation.page';
import { ActivityPagePage } from './features/activity/activity-page.page';

export const routes: Routes = [
  { path: '', component: DashboardPagePage, title: 'Workbench | TeachMyPC' },
  {
    path: 'rules',
    component: RuleCreationPage,
    title: 'Create rule | TeachMyPC',
    data: { title: 'Create rule', description: 'Describe a file task and review its safe preview.' },
  },
  {
    path: 'activity',
    component: ActivityPagePage,
    title: 'Activity | TeachMyPC',
  },
  { path: 'settings', component: SettingsPagePage, title: 'Settings | TeachMyPC' },
  { path: '**', redirectTo: '' },
];
