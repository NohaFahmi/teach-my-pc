import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';
import { AppInfoService } from '../../core/services/app-info.service';
import { SettingsPagePage } from './settings-page.page';

describe('SettingsPagePage', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [SettingsPagePage] }).compileComponents();
    const service = TestBed.inject(AppInfoService);
    vi.spyOn(service, 'load').mockResolvedValue(undefined);
    vi.spyOn(service, 'checkLocalAi').mockResolvedValue(undefined);
  });

  it('shows configured local AI details and offline guidance', () => {
    const service = TestBed.inject(AppInfoService);
    service.localAi.set({ status: 'unavailable', endpoint: 'http://127.0.0.1:11434', model: 'gemma3:4b', message: 'Start Ollama and install the model.' });
    const fixture = TestBed.createComponent(SettingsPagePage);
    fixture.detectChanges();
    const page = fixture.nativeElement as HTMLElement;
    expect(page.textContent).toContain('http://127.0.0.1:11434');
    expect(page.textContent).toContain('gemma3:4b');
    expect(page.textContent).toContain('Start Ollama and install the model.');
    fixture.destroy();
  });

  it('shows Connected only when the backend reports Ollama and its model available', () => {
    const service = TestBed.inject(AppInfoService);
    service.localAi.set({ status: 'available', endpoint: 'http://127.0.0.1:11434', model: 'gemma3:4b' });
    const fixture = TestBed.createComponent(SettingsPagePage);
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Connected');
    fixture.destroy();
  });

  it('shows the configured model as unavailable when Ollama is online without it', () => {
    const service = TestBed.inject(AppInfoService);
    service.localAi.set({ status: 'model-unavailable', endpoint: 'http://127.0.0.1:11434', model: 'gemma3:4b', message: 'Run ollama pull gemma3:4b.' });
    const fixture = TestBed.createComponent(SettingsPagePage);
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Configured model unavailable');
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Run ollama pull gemma3:4b.');
    fixture.destroy();
  });
});
