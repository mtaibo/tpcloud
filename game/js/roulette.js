/* Ruleta europea (37 números) */
(() => {
  const root = $('#view-roulette');
  const RED = new Set([1, 3, 5, 7, 9, 12, 14, 16, 18, 19, 21, 23, 25, 27, 30, 32, 34, 36]);
  const ORDER = [0, 32, 15, 19, 4, 21, 2, 25, 17, 34, 6, 27, 13, 36, 11, 30, 8, 23, 10, 5, 24, 16, 33, 1, 20, 14, 31, 9, 22, 18, 29, 7, 28, 12, 35, 3, 26];
  const SEG = (Math.PI * 2) / ORDER.length;
  const colorOf = n => (n === 0 ? 'green' : RED.has(n) ? 'red' : 'black');

  const PAY = { red: 1, black: 1, even: 1, odd: 1, low: 1, high: 1, d1: 2, d2: 2, d3: 2, c1: 2, c2: 2, c3: 2 };
  const wins = (key, n) => {
    if (key.startsWith('n:')) return n === +key.slice(2);
    switch (key) {
      case 'red': return RED.has(n);
      case 'black': return n > 0 && !RED.has(n);
      case 'even': return n > 0 && n % 2 === 0;
      case 'odd': return n % 2 === 1;
      case 'low': return n >= 1 && n <= 18;
      case 'high': return n >= 19;
      case 'd1': return n >= 1 && n <= 12;
      case 'd2': return n >= 13 && n <= 24;
      case 'd3': return n >= 25;
      case 'c1': return n > 0 && n % 3 === 1;
      case 'c2': return n > 0 && n % 3 === 2;
      case 'c3': return n > 0 && n % 3 === 0;
    }
    return false;
  };
  const payFor = key => (key.startsWith('n:') ? 35 : PAY[key]);

  let bets = {}, lastBets = null, undo = [], chip = 5, spinning = false, rot = 0;
  const history = [];

  /* ---------- Vista ---------- */
  const cell = (key, label, cls, style = '') =>
    `<div class="rl-cell ${cls}" data-bet="${key}" style="${style}">${label}</div>`;

  let board = cell('n:0', '0', 'green', 'grid-row:1/4;grid-column:1');
  for (let c = 0; c < 12; c++) for (let r = 0; r < 3; r++) {
    const n = c * 3 + (3 - r);
    board += cell('n:' + n, n, colorOf(n), `grid-row:${r + 1};grid-column:${c + 2}`);
  }
  board += cell('c3', '2:1', 'out', 'grid-row:1;grid-column:14');
  board += cell('c2', '2:1', 'out', 'grid-row:2;grid-column:14');
  board += cell('c1', '2:1', 'out', 'grid-row:3;grid-column:14');
  [['d1', '1ª docena'], ['d2', '2ª docena'], ['d3', '3ª docena']].forEach(([k, l], i) =>
    board += cell(k, l, 'out', `grid-row:4;grid-column:${2 + i * 4}/span 4`));
  [['low', '1-18', 'out'], ['even', 'PAR', 'out'], ['red', '◆ ROJO', 'red'], ['black', '◆ NEGRO', 'black'], ['odd', 'IMPAR', 'out'], ['high', '19-36', 'out']]
    .forEach(([k, l, cls], i) => board += cell(k, l, cls, `grid-row:5;grid-column:${2 + i * 2}/span 2`));

  root.innerHTML = `
    <div class="flex items-end justify-between mb-5 flex-wrap gap-2">
      <div>
        <h1 class="font-display text-3xl font-extrabold text-white">Ruleta Europea</h1>
        <p class="text-sm text-slate-400">37 números · pleno paga 35:1 · RTP 97,3 %</p>
      </div>
      <div id="rl-history" class="flex gap-1.5 flex-wrap"></div>
    </div>

    <div class="grid lg:grid-cols-[380px_1fr] gap-6">
      <div class="card-panel p-5 flex flex-col items-center">
        <div class="relative">
          <canvas id="rl-canvas" width="340" height="340" class="max-w-full"></canvas>
          <div id="rl-result" class="hidden absolute inset-0 m-auto w-24 h-24 rounded-full grid place-items-center text-4xl font-extrabold text-white result-banner border-4 border-gold shadow-glow"></div>
        </div>
        <div class="mt-4 w-full grid grid-cols-3 text-center gap-2">
          <div><div class="label-sm">Apuesta</div><div class="font-bold text-white tabular-nums" id="rl-total">0,00</div></div>
          <div><div class="label-sm">Último premio</div><div class="font-bold text-gold tabular-nums" id="rl-last">0,00</div></div>
          <div><div class="label-sm">Giros</div><div class="font-bold text-white" id="rl-count">0</div></div>
        </div>
        <button id="rl-spin" class="btn-gold w-full mt-4 text-base !py-3">GIRAR RULETA</button>
        <div class="grid grid-cols-3 gap-2 w-full mt-2">
          <button id="rl-undo" class="btn">↶ Deshacer</button>
          <button id="rl-clear" class="btn">Limpiar</button>
          <button id="rl-rebet" class="btn">↻ Repetir</button>
        </div>
      </div>

      <div class="card-panel p-5 min-w-0">
        <div class="felt p-4 overflow-x-auto"><div class="rl-board" id="rl-board">${board}</div></div>
        <div class="mt-5 flex items-center justify-between flex-wrap gap-3">
          <div>
            <div class="label-sm mb-2">Valor de la ficha</div>
            <div class="flex gap-3" id="rl-chips">${chipTrayHTML([1, 5, 10, 25, 100, 500], chip)}</div>
          </div>
          <button id="rl-double" class="btn">×2 Doblar apuestas</button>
        </div>
        <p class="text-xs text-slate-500 mt-4">Haz clic en el tablero para colocar fichas. Cada clic suma el valor de la ficha seleccionada.</p>
      </div>
    </div>`;

  const canvas = $('#rl-canvas', root), ctx = canvas.getContext('2d');
  const totalBet = () => Object.values(bets).reduce((a, b) => a + b, 0);

  /* ---------- Rueda ---------- */
  function drawWheel(rotation, ballAng, ballR) {
    const W = canvas.width, cx = W / 2, cy = W / 2, R = W / 2 - 8;
    ctx.clearRect(0, 0, W, W);
    // aro exterior
    const g = ctx.createRadialGradient(cx, cy, R - 20, cx, cy, R + 6);
    g.addColorStop(0, '#5b3a17'); g.addColorStop(1, '#2a1a08');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(cx, cy, R + 6, 0, 7); ctx.fill();
    ctx.strokeStyle = '#f5c451'; ctx.lineWidth = 2; ctx.stroke();

    ORDER.forEach((n, i) => {
      const a0 = rotation + i * SEG - Math.PI / 2 - SEG / 2, a1 = a0 + SEG;
      ctx.beginPath(); ctx.moveTo(cx, cy); ctx.arc(cx, cy, R - 6, a0, a1); ctx.closePath();
      ctx.fillStyle = { red: '#b91c1c', black: '#12161f', green: '#15803d' }[colorOf(n)];
      ctx.fill(); ctx.strokeStyle = 'rgba(245,196,81,.55)'; ctx.lineWidth = 1; ctx.stroke();
      // número
      ctx.save(); ctx.translate(cx, cy); ctx.rotate(a0 + SEG / 2);
      ctx.translate(R - 24, 0); ctx.rotate(Math.PI / 2);
      ctx.fillStyle = '#fff'; ctx.font = 'bold 12px Inter, sans-serif'; ctx.textAlign = 'center';
      ctx.fillText(n, 0, 0); ctx.restore();
    });
    // centro
    const gc = ctx.createRadialGradient(cx - 10, cy - 10, 5, cx, cy, R * 0.55);
    gc.addColorStop(0, '#3a2a14'); gc.addColorStop(1, '#17100a');
    ctx.fillStyle = gc; ctx.beginPath(); ctx.arc(cx, cy, R * 0.55, 0, 7); ctx.fill();
    ctx.strokeStyle = '#f5c451'; ctx.lineWidth = 2; ctx.stroke();
    ctx.fillStyle = '#f5c451'; ctx.font = '800 20px "Playfair Display", serif'; ctx.textAlign = 'center';
    ctx.fillText('♛', cx, cy + 7);
    // bola
    if (ballAng !== null) {
      const bx = cx + Math.cos(ballAng) * ballR, by = cy + Math.sin(ballAng) * ballR;
      ctx.fillStyle = '#fff'; ctx.shadowColor = 'rgba(0,0,0,.8)'; ctx.shadowBlur = 6;
      ctx.beginPath(); ctx.arc(bx, by, 6, 0, 7); ctx.fill(); ctx.shadowBlur = 0;
    }
  }
  drawWheel(rot, null, 0);

  function animateSpin(idx) {
    const dur = 5200;
    const target = -idx * SEG;
    const delta = ((target - rot) % (Math.PI * 2) + Math.PI * 2) % (Math.PI * 2) + Math.PI * 2 * 4;
    const start = rot, t0 = performance.now(), R = canvas.width / 2 - 8;
    return new Promise(res => {
      const step = now => {
        const t = Math.min(1, (now - t0) / dur), e = 1 - Math.pow(1 - t, 3);
        rot = start + delta * e;
        const ballAng = -Math.PI / 2 - (1 - e) * Math.PI * 14;
        drawWheel(rot, ballAng, R - 14 - t * 12);
        t < 1 ? requestAnimationFrame(step) : res();
      };
      requestAnimationFrame(step);
    });
  }

  /* ---------- Apuestas ---------- */
  function renderChips() {
    $$('.chip-on', root).forEach(e => e.remove());
    for (const [key, amt] of Object.entries(bets)) {
      const el = $(`[data-bet="${key}"]`, root);
      if (!el) continue;
      const c = document.createElement('span'); c.className = 'chip-on'; c.textContent = amt >= 1000 ? (amt / 1000) + 'k' : amt;
      el.appendChild(c);
    }
    $('#rl-total', root).textContent = fmt(totalBet());
    $('#rl-spin', root).disabled = spinning || totalBet() === 0;
  }
  function addBet(key, amt) {
    if (!Wallet.debit(amt)) return false;
    bets[key] = (bets[key] || 0) + amt; undo.push([key, amt]); return true;
  }
  function refundAll() { const t = totalBet(); if (t) Wallet.refund(t); }

  function renderHistory() {
    $('#rl-history', root).innerHTML = history.slice(-12).reverse()
      .map(n => `<span class="num-pill ${colorOf(n)}">${n}</span>`).join('');
  }

  $('#rl-board', root).addEventListener('click', e => {
    const c = e.target.closest('[data-bet]'); if (!c || spinning) return;
    if (addBet(c.dataset.bet, chip)) renderChips();
  });
  bindChipTray($('#rl-chips', root), v => chip = v);
  $('#rl-clear', root).onclick = () => { if (spinning) return; refundAll(); bets = {}; undo = []; renderChips(); };
  $('#rl-undo', root).onclick = () => {
    if (spinning || !undo.length) return;
    const [k, a] = undo.pop(); bets[k] -= a; if (bets[k] <= 0) delete bets[k];
    Wallet.refund(a); renderChips();
  };
  $('#rl-rebet', root).onclick = () => {
    if (spinning || !lastBets) return;
    for (const [k, a] of Object.entries(lastBets)) if (!addBet(k, a)) break;
    renderChips();
  };
  $('#rl-double', root).onclick = () => {
    if (spinning || !totalBet()) return;
    for (const [k, a] of Object.entries({ ...bets })) if (!addBet(k, a)) break;
    renderChips();
  };

  $('#rl-spin', root).onclick = async () => {
    if (spinning || !totalBet()) return;
    spinning = true; lastBets = { ...bets };
    $$('.rl-cell.hit', root).forEach(c => c.classList.remove('hit'));
    $('#rl-result', root).classList.add('hidden');
    renderChips();
    const idx = randInt(0, ORDER.length - 1), n = ORDER[idx];
    await animateSpin(idx);

    let ret = 0;
    for (const [k, a] of Object.entries(bets)) if (wins(k, n)) ret += a * (payFor(k) + 1);
    const staked = totalBet();
    Wallet.credit(ret); Wallet.log('Ruleta', staked, ret);
    history.push(n); renderHistory();
    const res = $('#rl-result', root);
    res.textContent = n; res.className = `absolute inset-0 m-auto w-24 h-24 rounded-full grid place-items-center text-4xl font-extrabold text-white result-banner border-4 border-gold shadow-glow ${colorOf(n) === 'red' ? 'bg-red-700' : colorOf(n) === 'green' ? 'bg-green-700' : 'bg-slate-900'}`;
    $(`[data-bet="n:${n}"]`, root).classList.add('hit');
    $('#rl-last', root).textContent = fmt(ret);
    $('#rl-count', root).textContent = history.length;
    if (ret > staked) Toast.show(`¡Sale el ${n}! Ganas ${fmt(ret - staked)} cr`, 'win');
    else if (ret > 0) Toast.show(`Sale el ${n}. Recuperas ${fmt(ret)} cr`);
    else Toast.show(`Sale el ${n}. Sin premio`, 'lose');
    bets = {}; undo = []; spinning = false; renderChips();
  };

  renderChips();
})();
