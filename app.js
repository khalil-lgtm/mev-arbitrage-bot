const STATUS = document.getElementById('status');
const TBODY = document.getElementById('body');

function fmtAddr(a) {
  return a ? a.slice(0, 6) + '…' + a.slice(-4) : '—';
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
}

function row(opp, i) {
  const tr = document.createElement('tr');
  const pools = (opp.pools || []).map(fmtAddr).join(', ') || '—';
  tr.innerHTML = `
    <td>${i + 1}</td>
    <td>${escapeHtml(opp.ts || '—')}</td>
    <td>${fmtAddr(opp.tx_hash)}</td>
    <td>${escapeHtml(opp.protocol || '—')}</td>
    <td>${opp.hops != null ? opp.hops : '—'}</td>
    <td>${fmtAddr(opp.base_token)}</td>
    <td class="profit">${opp.gross_profit_eth != null ? opp.gross_profit_eth.toFixed(6) : '—'}</td>
    <td>${opp.optimal_loan_eth != null ? opp.optimal_loan_eth.toFixed(4) : '—'}</td>
    <td>${opp.confidence != null ? (opp.confidence * 100).toFixed(1) + '%' : '—'}</td>
    <td title="${escapeHtml(pools)}">${escapeHtml(pools)}</td>
  `;
  return tr;
}

async function load() {
  STATUS.textContent = 'loading…';
  try {
    const res = await fetch('opportunities.jsonl', { cache: 'no-store' });
    if (!res.ok) throw new Error(res.status);
    const text = await res.text();
    let items = parseOpportunities(text);
    TBODY.innerHTML = '';
    if (!items.length) {
      const tr = document.createElement('tr');
      tr.innerHTML = `<td colspan="10" class="empty">No opportunities recorded yet.</td>`;
      TBODY.appendChild(tr);
    } else {
      // Newest first
      items.slice().reverse().forEach((o, i) => TBODY.appendChild(row(o, i)));
    }
    STATUS.textContent = `${items.length} opportunity${items.length === 1 ? '' : 'ies'} shown`;
  } catch (e) {
    STATUS.textContent = 'failed: ' + e.message;
  }
}

// Accept either a JSON array or JSONL (one record per line).
function parseOpportunities(text) {
  const trimmed = text.trim();
  if (!trimmed) return [];
  try {
    const arr = JSON.parse(trimmed);
    return Array.isArray(arr) ? arr : [];
  } catch (_) {
    return trimmed
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        try { return JSON.parse(l); } catch (_) { return null; }
      })
      .filter((x) => x != null);
  }
}

load();
setInterval(load, 30000);
