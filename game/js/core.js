/* Utilidades compartidas: monedero (créditos ficticios), toasts, cartas */
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];
const sleep = ms => new Promise(r => setTimeout(r, ms));
/** Aleatorio uniforme [0,1) con CSPRNG del navegador. */
const rnd = () => { const a = new Uint32Array(1); crypto.getRandomValues(a); return a[0] / 4294967296; };
const rand = (a, b) => a + Math.random() * (b - a);
const randInt = (a, b) => Math.floor(rand(a, b + 1));
const pick = arr => arr[Math.floor(Math.random() * arr.length)];
const fmt = n => n.toLocaleString('es-ES', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/* ---------- Monedero ---------- */
const Wallet = (() => {
  const KEY = 'royalbet.wallet.v1';
  const START = 1000;
  let s = { balance: START, wagered: 0, won: 0, rounds: 0, history: [], best: 0 };
  try { s = { ...s, ...JSON.parse(localStorage.getItem(KEY) || '{}') }; } catch (e) { /* sin storage */ }
  const listeners = [];
  const save = () => {
    try { localStorage.setItem(KEY, JSON.stringify(s)); } catch (e) { /* ignorar */ }
    listeners.forEach(fn => fn(s));
  };
  return {
    get state() { return s; },
    get balance() { return s.balance; },
    canAfford: a => a > 0 && s.balance + 1e-9 >= a,
    /** Descuenta una apuesta. Devuelve false si no hay saldo. */
    debit(a) {
      if (!(a > 0) || s.balance + 1e-9 < a) { Toast.show('Saldo insuficiente', 'lose'); return false; }
      s.balance = Math.round((s.balance - a) * 100) / 100;
      s.wagered += a; save(); return true;
    },
    /** Abona un premio (importe total devuelto, incluida la apuesta). */
    credit(a) {
      if (a > 0) { s.balance = Math.round((s.balance + a) * 100) / 100; s.won += a; }
      save();
    },
    /** Devuelve una apuesta retirada antes de jugar (no cuenta como premio). */
    refund(a) { s.balance = Math.round((s.balance + a) * 100) / 100; s.wagered -= a; save(); },
    /** Registra una ronda terminada: stake apostado y ret devuelto (ya abonado con credit). */
    log(game, stake, ret) {
      s.rounds++;
      s.history.unshift({ t: Date.now(), game, stake, ret });
      s.history = s.history.slice(0, 40);
      if (ret - stake > s.best) s.best = ret - stake;
      save();
    },
    resetAll() { s = { balance: START, wagered: 0, won: 0, rounds: 0, history: [], best: 0 }; save(); },
    round() { s.rounds++; save(); },
    reload(a = 1000) { s.balance = Math.round((s.balance + a) * 100) / 100; save(); },
    onChange(fn) { listeners.push(fn); fn(s); },
  };
})();

/* ---------- Sonido (WebAudio, sin archivos) ---------- */
const Sound = (() => {
  let ctx = null, on = true;
  try { on = localStorage.getItem('royalbet.sound') !== '0'; } catch (e) { /* ignorar */ }
  const tone = (f, d, type = 'sine', v = 0.06, delay = 0) => {
    if (!on) return;
    try {
      ctx = ctx || new (window.AudioContext || window.webkitAudioContext)();
      const o = ctx.createOscillator(), g = ctx.createGain(), t = ctx.currentTime + delay;
      o.type = type; o.frequency.setValueAtTime(f, t);
      g.gain.setValueAtTime(v, t); g.gain.exponentialRampToValueAtTime(0.0001, t + d);
      o.connect(g); g.connect(ctx.destination); o.start(t); o.stop(t + d);
    } catch (e) { /* audio no disponible */ }
  };
  const sounds = {
    click: () => tone(520, 0.05, 'triangle', 0.04),
    tick: () => tone(900, 0.03, 'square', 0.02),
    win: () => [523, 659, 784, 1046].forEach((f, i) => tone(f, 0.22, 'triangle', 0.06, i * 0.09)),
    big: () => [523, 659, 784, 1046, 1318, 1568].forEach((f, i) => tone(f, 0.3, 'triangle', 0.07, i * 0.08)),
    lose: () => [300, 220].forEach((f, i) => tone(f, 0.25, 'sawtooth', 0.04, i * 0.12)),
    boom: () => { tone(110, 0.4, 'sawtooth', 0.09); tone(70, 0.5, 'square', 0.07, 0.05); },
    gem: () => tone(880 + Math.random() * 200, 0.12, 'sine', 0.06),
    cash: () => [880, 1320].forEach((f, i) => tone(f, 0.15, 'triangle', 0.06, i * 0.07)),
  };
  return {
    get on() { return on; },
    toggle() { on = !on; try { localStorage.setItem('royalbet.sound', on ? '1' : '0'); } catch (e) { /* ignorar */ } return on; },
    play(n) { (sounds[n] || (() => {}))(); },
  };
})();

/* ---------- Toasts ---------- */
const Toast = {
  show(msg, type = '') {
    if (type === 'win') Sound.play('win'); else if (type === 'lose') Sound.play('lose');
    const el = document.createElement('div');
    el.className = 'toast ' + type;
    el.textContent = msg;
    $('#toasts').appendChild(el);
    setTimeout(() => { el.classList.add('out'); setTimeout(() => el.remove(), 250); }, 3200);
  },
};

/* ---------- Baraja ---------- */
const SUITS = ['♠', '♥', '♦', '♣'];
const rankLabel = r => ({ 11: 'J', 12: 'Q', 13: 'K', 14: 'A' }[r] || String(r));
function newDeck(decks = 1) {
  const d = [];
  for (let k = 0; k < decks; k++) for (const s of SUITS) for (let r = 2; r <= 14; r++) d.push({ r, s });
  for (let i = d.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [d[i], d[j]] = [d[j], d[i]]; }
  return d;
}
function cardEl(c, hidden = false) {
  const d = document.createElement('div');
  if (hidden || !c) { d.className = 'pcard back'; return d; }
  const red = c.s === '♥' || c.s === '♦';
  d.className = 'pcard ' + (red ? 'red' : 'black');
  d.innerHTML = `<span class="cr">${rankLabel(c.r)}</span><span class="cs">${c.s}</span><span class="cr b">${rankLabel(c.r)}</span>`;
  return d;
}
function emptyCardEl() { const d = document.createElement('div'); d.className = 'pcard empty'; return d; }

/* ---------- Selector de fichas reutilizable ---------- */
function chipTrayHTML(values = [1, 5, 10, 25, 100, 500], selected = 5) {
  return values.map(v => `<button class="chip ${v === selected ? 'sel' : ''}" data-v="${v}" aria-label="Ficha ${v}">${v}</button>`).join('');
}
function bindChipTray(root, onPick) {
  root.addEventListener('click', e => {
    const b = e.target.closest('.chip'); if (!b) return;
    $$('.chip', root).forEach(c => c.classList.toggle('sel', c === b));
    onPick(+b.dataset.v);
  });
}

/* ---------- Control de importe reutilizable (input + ½ / 2× / Máx) ---------- */
function betBoxHTML(id, def = 10) {
  return `<div>
    <label class="label-sm" for="${id}">Importe de la apuesta</label>
    <div class="flex gap-1 mt-1.5">
      <input id="${id}" type="number" min="1" step="1" value="${def}" class="input" inputmode="decimal">
      <button class="btn !px-3" data-bet-op="half" data-for="${id}" title="Mitad">½</button>
      <button class="btn !px-3" data-bet-op="double" data-for="${id}" title="Doble">2×</button>
      <button class="btn !px-3" data-bet-op="max" data-for="${id}" title="Todo el saldo">Máx</button>
    </div></div>`;
}
document.addEventListener('click', e => {
  const b = e.target.closest('[data-bet-op]'); if (!b) return;
  const inp = document.getElementById(b.dataset.for); if (!inp || inp.disabled) return;
  const v = +inp.value || 0;
  const n = b.dataset.betOp === 'half' ? v / 2 : b.dataset.betOp === 'double' ? v * 2 : Wallet.balance;
  inp.value = Math.max(1, Math.floor(Math.min(n, Wallet.balance) * 100) / 100);
  inp.dispatchEvent(new Event('input', { bubbles: true }));
  Sound.play('click');
});
/** Lee y valida el importe de un input de apuesta (o null). */
function readBet(id) {
  const v = Math.round((+document.getElementById(id).value || 0) * 100) / 100;
  if (v < 1) { Toast.show('La apuesta mínima es 1', 'lose'); return null; }
  if (!Wallet.canAfford(v)) { Toast.show('Saldo insuficiente', 'lose'); return null; }
  return v;
}
/** Activa/desactiva un conjunto de controles. */
const setDisabled = (els, d) => els.forEach(e => e && (e.disabled = d));
/** Cabecera estándar de un juego. */
const gameHeader = (title, sub, rtp) => `
  <div class="flex items-end justify-between mb-5 flex-wrap gap-2">
    <div>
      <h1 class="font-display text-3xl font-extrabold text-white">${title}</h1>
      <p class="text-sm text-slate-400">${sub}</p>
    </div>
    ${rtp ? `<span class="text-xs font-bold text-emerald-300 bg-emerald-900/30 border border-emerald-700/50 rounded-full px-3 py-1">RTP ${rtp}</span>` : ''}
  </div>`;
