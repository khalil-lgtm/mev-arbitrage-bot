const STATUS = document.getElementById('status');
const TBODY = document.getElementById('body');

function fmtAddr(a) { return a ? a.slice(0, 6) + '…' + a.slice(-4) : '—'; }
function n(x, d = 6) { return (x == null) ? '—' : Number(x).toFixed(d); }
function pct(x) { return (x == null) ? '—' : ((x * 100).toFixed(1) + '%'); }

// hh:mm dd/mm format from Date in UTC (fixed reference). Browser clock otherwise.
function fmtAge(tsISO, now = Date.now()) {
  const t = new Date(tsISO).getTime();
  if (!t) return '—';
  const h = Math.floor((now - t) / 3600000);
  const m = Math.floor(((now - t) % 3600000) / 60000);
  return `${h}h ${m}m ago`;
}

async function load() {
  STATUS.textContent = 'loading…';
  try {
    const res = await fetch('opportunities.jsonl', { cache: 'no-store' });
    if (!res.ok) throw new Error(res.status);
    const text = await res.text();
    const items = parseJSONL(text);
    build(items);
  } catch (e) {
    STATUS.textContent = 'failed: ' + e.message;
  }
}

function parseJSONL(text) {
  return text.trim().split(/\r?\n/).map(l => l.trim()).filter(Boolean)
    .map(l => { try { return JSON.parse(l); } catch (_) { return null; } })
    .filter(x => x != null);
}

function groupBy(arr, key) {
  const g = {};
  arr.forEach(o => { (g[o[key]] = g[o[key]] || []).push(o); });
  return g;
}

function sum(arr, k) { return arr.reduce((a, o) => a + Number(o[k] || 0), 0); }
function avg(arr, k) { return arr.length ? (sum(arr, k) / arr.length) : 0; }
function latestTs(arr) { return arr.map(o => o.ts).filter(Boolean).sort().slice(-1)[0]; }

function build(items) {
  TBODY.innerHTML = '';
  const groups = groupBy(items, 'protocol');
  const now = Date.now();

  let i = 1;
  let totalGross = 0, totalNet = 0;
  let latestOverall = '';

  for (const [proto, list] of Object.entries(groups)) {
    const g = list.length;
    const gross = sum(list, 'gross_profit_eth');
    const net = sum(list, 'net_eth');
    const conf = avg(list, 'confidence');
    const hops = avg(list, 'hops');
    const last = latestTs(list) || '—';
    const age = last === '—' ? '—' : fmtAge(last, now);

    totalGross += gross; totalNet += net;
    if (last && last > latestOverall) latestOverall = last;

    const tr = document.createElement('tr');
    tr.innerHTML = `
      <td>${i++}</td>
      <td>${proto}</td>
      <td>${g}</td>
      <td>${n(gross, 6)}</td>
      <td>${n(net, 6)}</td>
      <td>${pct(conf)}</td>
      <td>${hops.toFixed(2)}</td>
      <td>${last}</td>
      <td>${age}</td>
    `;
    TBODY.appendChild(tr);
  }

  if (!items.length) {
    const tr = document.createElement('tr');
    tr.innerHTML = `<td colspan="9" class="empty">No opportunities recorded yet.</td>`;
    TBODY.appendChild(tr);
  }

  document.getElementById('k-strat').textContent = Object.keys(groups).length || 0;
  document.getElementById('k-routes').textContent = items.length || 0;
  document.getElementById('k-gross').textContent = n(totalGross, 6);
  document.getElementById('k-net').textContent = n(totalNet, 6);
  document.getElementById('k-latest').textContent = latestOverall || '—';
  STATUS.textContent = `${items.length} route${items.length === 1 ? '' : 's'} · ${Object.keys(groups).length} strateg${Object.keys(groups).length === 1 ? 'y' : 'ies'}`;
}

load();
setInterval(load, 30000);
