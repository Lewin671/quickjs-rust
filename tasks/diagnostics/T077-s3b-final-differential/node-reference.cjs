globalThis.print=console.log;
try { require('vm').runInThisContext(require('fs').readFileSync(process.argv[2],'utf8')); }
catch(e){ console.error(e&&e.name ? e.name+': '+e.message : String(e));process.exit(1); }
