'use strict';
// Runs one Nuvio scraper (a JavaScript file exposing getStreams) and prints
// exactly one JSON line to stdout: {"streams":[...]} or {"error":"..."}.
//
//   node nuvio_runner.cjs <scraper.js> <id> <movie|tv> <season|-> <episode|->
//
// Anything the scraper prints goes to stderr so stdout stays machine-readable.

const fs = require('fs');
const path = require('path');
const vm = require('vm');
const Module = require('module');
const { pathToFileURL } = require('url');

const realWrite = process.stdout.write.bind(process.stdout);
const toStderr = (...args) => {
  try {
    process.stderr.write(args.map((a) => (typeof a === 'string' ? a : JSON.stringify(a))).join(' ') + '\n');
  } catch (_) {
    /* ignore */
  }
};
console.log = console.info = console.debug = console.warn = toStderr;
process.stdout.write = (chunk, ...rest) => process.stderr.write(chunk, ...rest);

let finished = false;
function finish(result, code) {
  if (finished) return;
  finished = true;
  realWrite(JSON.stringify(result) + '\n', () => process.exit(code));
}

const timeoutMs = Number(process.env.NUVIO_TIMEOUT_MS) || 35000;
// Not unref'd: a scraper whose promise never settles must still be reported.
setTimeout(() => finish({ error: 'scraper timed out' }, 1), timeoutMs);

process.on('uncaughtException', (e) => finish({ error: 'uncaught: ' + (e && e.message ? e.message : String(e)) }, 1));
process.on('unhandledRejection', (e) => toStderr('unhandled rejection:', e && e.message ? e.message : String(e)));

// Scrapers expect a browser-like fetch: plain "node" user agents are often
// rejected, so one is added unless the scraper sets its own.
const BROWSER_UA =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36';
if (typeof globalThis.atob !== 'function') {
  globalThis.atob = (b64) => Buffer.from(String(b64), 'base64').toString('binary');
}
if (typeof globalThis.btoa !== 'function') {
  globalThis.btoa = (bin) => Buffer.from(String(bin), 'binary').toString('base64');
}

const nativeFetch = globalThis.fetch;
if (typeof nativeFetch === 'function') {
  globalThis.fetch = (input, init) => {
    const opts = Object.assign({}, init || {});
    const headers = new Headers(opts.headers || (input && input.headers) || {});
    if (!headers.has('user-agent')) headers.set('user-agent', BROWSER_UA);
    opts.headers = headers;
    if (!opts.redirect) opts.redirect = 'follow';
    if (!opts.signal && typeof AbortSignal !== 'undefined' && AbortSignal.timeout) {
      opts.signal = AbortSignal.timeout(25000);
    }
    return nativeFetch(input, opts);
  };
}

function pickGetStreams(mod, ns) {
  const candidates = [
    mod && mod.exports,
    mod && mod.exports && mod.exports.default,
    ns,
    ns && ns.default,
    globalThis,
  ];
  for (const c of candidates) {
    if (c && typeof c.getStreams === 'function') return c.getStreams.bind(c);
  }
  return null;
}

async function loadScraper(file) {
  const source = fs.readFileSync(file, 'utf8');
  const mod = { exports: {} };
  const req = Module.createRequire(file);
  try {
    const wrapper = vm.runInThisContext(
      '(function (exports, require, module, __filename, __dirname) {' + source + '\n})',
      { filename: file }
    );
    wrapper.call(mod.exports, mod.exports, req, mod, file, path.dirname(file));
    const fn = pickGetStreams(mod, null);
    if (fn) return fn;
    throw new Error('the script does not define getStreams');
  } catch (e) {
    const msg = String(e && e.message ? e.message : e);
    const esmSyntax = e instanceof SyntaxError && /import|export|module/i.test(msg);
    if (!esmSyntax) throw e;
  }
  // Modern scrapers may be ES modules: load a PID-scoped .mjs copy so
  // concurrent runs of the same scraper do not unlink each other's module.
  const copy = file.replace(/\.[^.]*$/, '') + '.' + process.pid + '.run.mjs';
  fs.writeFileSync(copy, source);
  try {
    const ns = await import(pathToFileURL(copy).href);
    const fn = pickGetStreams(null, ns);
    if (!fn) throw new Error('the module does not export getStreams');
    return fn;
  } finally {
    try {
      fs.unlinkSync(copy);
    } catch (_) {
      /* ignore */
    }
  }
}

function text(v) {
  return v === undefined || v === null ? '' : String(v);
}

function normalize(out) {
  let list = out;
  if (list && !Array.isArray(list) && Array.isArray(list.streams)) list = list.streams;
  if (!Array.isArray(list)) return [];
  const streams = [];
  for (const s of list) {
    if (!s || typeof s !== 'object' || s.isDirect === false) continue;
    const url = text(s.url || s.streamUrl).trim();
    if (!/^https?:\/\//i.test(url)) continue;
    const headers = {};
    if (s.headers && typeof s.headers === 'object') {
      for (const [k, v] of Object.entries(s.headers)) {
        if (typeof v === 'string' || typeof v === 'number') headers[k] = String(v);
      }
    }
    streams.push({
      name: text(s.name),
      title: text(s.title),
      url,
      quality: text(s.quality),
      size: text(s.size),
      headers,
    });
  }
  return streams;
}

async function main() {
  const [file, id, mediaType, seasonArg, episodeArg] = process.argv.slice(2);
  if (!file || !id || !mediaType) {
    finish({ error: 'usage: nuvio_runner.cjs <scraper.js> <id> <movie|tv> <season|-> <episode|->' }, 2);
    return;
  }
  const num = (v) => (v === undefined || v === '-' || v === '' || Number.isNaN(Number(v)) ? null : Number(v));
  let userConfig = {};
  if (process.env.MOVIEBOX_NUVIO_CONFIG) {
    try {
      const parsed = JSON.parse(process.env.MOVIEBOX_NUVIO_CONFIG);
      if (parsed && typeof parsed === 'object') userConfig = parsed;
    } catch (_) {
      /* ignore invalid JSON */
    }
  }
  const getStreams = await loadScraper(path.resolve(file));
  const result = await getStreams(id, mediaType, num(seasonArg), num(episodeArg), userConfig);
  finish({ streams: normalize(result) }, 0);
}

main().catch((e) => finish({ error: String(e && e.message ? e.message : e) }, 1));
