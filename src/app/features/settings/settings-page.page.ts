import { Component, inject, OnInit } from '@angular/core';
import { AppInfoService } from '../../core/services/app-info.service';

@Component({
  selector: 'app-settings-page',
  templateUrl: 'settings-page.page.html',
  styleUrls: ['settings-page.page.scss']
})
export class SettingsPagePage implements OnInit {
  protected readonly appInfo = inject(AppInfoService);

  ngOnInit(): void {
    void this.appInfo.load();
    void this.appInfo.checkLocalAi();
  }
}
