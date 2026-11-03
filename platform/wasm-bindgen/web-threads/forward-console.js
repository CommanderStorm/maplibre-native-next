// Workers' console output also goes to the page (where test tools read it).
for (const level of ["log", "warn", "error"]) {
  const original = console[level].bind(console);
  console[level] = (...args) => {
    original(...args);
    postMessage({ console: level, text: args.map((a) => (a instanceof Error ? a.stack : String(a))).join(" ") });
  };
}
