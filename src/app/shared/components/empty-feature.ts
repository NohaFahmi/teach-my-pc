import { Component, input } from '@angular/core';

@Component({
  selector: 'app-empty-feature-page',
  templateUrl: './empty-feature-page.html',
  styleUrl: './empty-feature-page.scss',
})
export class EmptyFeaturePage {
  readonly title = input.required<string>();
  readonly description = input.required<string>();
}
