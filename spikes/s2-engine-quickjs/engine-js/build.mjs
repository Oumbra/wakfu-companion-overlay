// Bundle + transpile `entry.ts` (et ses deux dépendances vendées) en un unique script IIFE
// ES2020, sans aucune dépendance externe restante — c'est ce script que le harnais Rust
// charge dans QuickJS. `target: es2020` : QuickJS ne suit pas les moteurs V8/JSC sur le
// support des toutes dernières syntaxes ; es2020 (optional chaining, nullish coalescing,
// pas de champs privés `#`) est confortablement dans son périmètre et suffit au code vendu
// (vérifié : log-parser.ts n'utilise ni `#champ`, ni groupes nommés dans ses regex).
import { build } from 'esbuild';

await build({
  entryPoints: ['src/entry.ts'],
  bundle: true,
  format: 'iife',
  target: 'es2020',
  platform: 'neutral',
  outfile: 'dist/engine.bundle.js',
  legalComments: 'none',
});

console.log('OK dist/engine.bundle.js');
