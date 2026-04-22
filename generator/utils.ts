import * as fs from 'fs';
import * as path from 'path';

/** Recursively copy a directory. */
export function copyDir(src: string, dest: string): void {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDir(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

/** Replace all {{KEY}} placeholders in a template string. */
export function fillTemplate(template: string, values: Record<string, string>): string {
  return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
    if (key in values) return values[key];
    throw new Error(`Unknown template placeholder: ${match}`);
  });
}
