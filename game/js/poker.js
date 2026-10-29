/* Texas Hold'em heads-up contra la casa (apuesta fija = ante) */
(() => {
  const root = $('#view-poker');
  const HAND_NAMES = ['Carta alta', 'Pareja', 'Doble pareja', 'Trío', 'Escalera', 'Color', 'Full house', 'Póker', 'Escalera de color'];
  const STAGES = ['Pre-flop', 'Flop', 'Turn', 'River'];

  /* ---------- Evaluador ---------- */
  function eval5(c) {
    const r = c.map(x => x.r).sort((a, b) => b - a);
    const flush = c.every(x => x.s === c[0].s);
    let sh = 0;
    if (new Set(r).size === 5) {
      if (r[0] - r[4] === 4) sh = r[0];
      else if (r[0] === 14 && r[1] === 5) sh = 5;
    }
    const cnt = {}; r.forEach(x => cnt[x] = (cnt[x] || 0) + 1);
    const g = Object.entries(cnt).map(([k, v]) => [+k, v]).sort((a, b) => b[1] - a[1] || b[0] - a[0]);
    const ranks = g.map(x => x[0]);
    if (sh && flush) return [8, sh];
    if (g[0][1] === 4) return [7, ...ranks];
    if (g[0][1] === 3 && g[1][1] === 2) return [6, ...ranks];
    if (flush) return [5, ...r];
    if (sh) return [4, sh];
    if (g[0][1] === 3) return [3, ...ranks];
    if (g[0][1] === 2 && g[1][1] === 2) return [2, ...ranks];
    if (g[0][1] === 2) return [1, ...ranks];
    return [0, ...r];
  }
  const cmp = (a, b) => { for (let i = 0; i < Math.max(a.length, b.length); i++) { const d = (a[i] || 0) - (b[i] || 0); if (d) return d; } return 0; };
  function best(cards) {
    let top = null, topCards = null;
    const n = cards.length;
    for (let a = 0; a < n - 4; a++) for (let b = a + 1; b < n - 3; b++) for (let c = b + 1; c < n - 2; c++)
      for (let d = c + 1; d < n - 1; d++) for (let e = d + 1; e < n; e++) {
        const hand = [cards[a], cards[b], cards[c], cards[d], cards[e]], v = eval5(hand);
        if (!top || cmp(v, top) > 0) { top = v; topCards = hand; }
      }
    return { v: top, cards: topCards };
  }

  /* ---------- Estado ---------- */
  let st = { phase: 'idle', deck: [], me: [], bot: [], board: [], pot: 0, ante: 10, stage: 0, facing: false, msg: '' };
  let log = [];

  root.innerHTML = `
    <div class="flex items-end justify-between mb-5 flex-wrap gap-2">
      <div>
        <h1 class="font-display text-3xl font-extrabold text-white">Texas Hold'em</h1>
        <p class="text-sm text-slate-400">Cara a cara contra la casa · Apuestas de tamaño fijo en cada ronda</p>
      </div>
    </div>
    <div class="grid lg:grid-cols-[1fr_300px] gap-5">
      <div>
        <div class="felt p-5 sm:p-8">
          <div class="text-center">
            <div class="label-sm !text-emerald-200/70 mb-2">La casa <span id="pk-bothand" class="ml-1 text-gold-light normal-case tracking-normal"></span></div>
            <div id="pk-bot" class="flex justify-center gap-2 min-h-[100px]"></div>
          </div>
          <div class="my-5 text-center">
            <div class="inline-flex items-center gap-3 bg-black/40 rounded-full px-5 py-1.5 mb-3">
              <span class="label-sm !text-emerald-200/70" id="pk-stage">Pre-flop</span>
              <span class="text-gold font-extrabold tabular-nums">Bote <span id="pk-pot">0,00</span></span>
            </div>
            <div id="pk-board" class="flex justify-center gap-2 flex-wrap"></div>
            <div id="pk-msg" class="mt-3 min-h-[32px] font-display text-xl font-extrabold text-gold-light"></div>
          </div>
          <div class="text-center">
            <div id="pk-me" class="flex justify-center gap-2 min-h-[100px]"></div>
            <div class="label-sm !text-emerald-200/70 mt-2">Tú <span id="pk-myhand" class="ml-1 text-gold-light normal-case tracking-normal"></span></div>
          </div>
        </div>

        <div class="card-panel mt-5 p-5 flex flex-wrap items-center justify-between gap-4">
          <div id="pk-ante-ui">
            <div class="label-sm mb-2">Ante / apuesta por ronda: <span class="text-gold text-sm" id="pk-ante">10,00</span></div>
            <div class="flex gap-3" id="pk-chips">${chipTrayHTML([5, 10, 25, 100, 500], 10)}</div>
          </div>
          <div class="flex gap-2 flex-wrap" id="pk-actions"></div>
        </div>
      </div>

      <aside class="card-panel p-5">
        <h3 class="font-bold text-white mb-3">Historial de la mano</h3>
        <ul id="pk-log" class="text-sm text-slate-400 space-y-1.5 max-h-72 overflow-y-auto"></ul>
        <h3 class="font-bold text-white mt-5 mb-2">Manos</h3>
        <ol class="text-xs text-slate-500 space-y-0.5 list-decimal list-inside">
          <li>Escalera de color</li><li>Póker</li><li>Full house</li><li>Color</li><li>Escalera</li>
          <li>Trío</li><li>Doble pareja</li><li>Pareja</li><li>Carta alta</li>
        </ol>
      </aside>
    </div>`;

  const E = {
    bot: $('#pk-bot', root), me: $('#pk-me', root), board: $('#pk-board', root), pot: $('#pk-pot', root),
    stage: $('#pk-stage', root), msg: $('#pk-msg', root), actions: $('#pk-actions', root), log: $('#pk-log', root),
    my: $('#pk-myhand', root), bh: $('#pk-bothand', root),
  };

  const say = t => { log.unshift(t); log = log.slice(0, 30); E.log.innerHTML = log.map(l => `<li>${l}</li>`).join(''); };

  function render(showBot = false, winCards = null) {
    const isWin = c => winCards && winCards.includes(c);
    const mk = (c, hide) => { const el = cardEl(c, hide); if (isWin(c)) el.classList.add('win'); return el; };
    E.bot.replaceChildren(...(st.bot.length ? st.bot.map(c => mk(c, !showBot)) : [emptyCardEl(), emptyCardEl()]));
    E.me.replaceChildren(...(st.me.length ? st.me.map(c => mk(c, false)) : [emptyCardEl(), emptyCardEl()]));
    const b = [...st.board.map(c => mk(c, false))];
    while (b.length < 5) b.push(emptyCardEl());
    E.board.replaceChildren(...b);
    E.pot.textContent = fmt(st.pot);
    E.stage.textContent = st.phase === 'idle' ? 'Esperando' : st.phase === 'done' ? 'Showdown' : STAGES[st.stage];
    E.msg.textContent = st.msg;
    E.my.textContent = st.me.length + st.board.length >= 5 ? '· ' + HAND_NAMES[best([...st.me, ...st.board]).v[0]] : '';
    E.bh.textContent = showBot && st.board.length === 5 ? '· ' + HAND_NAMES[best([...st.bot, ...st.board]).v[0]] : '';
    renderActions();
  }

  function renderActions() {
    const A = st.ante, locked = st.phase === 'play';
    $('#pk-ante-ui', root).style.opacity = locked ? .4 : 1;
    $('#pk-ante-ui', root).style.pointerEvents = locked ? 'none' : '';
    if (st.phase === 'idle' || st.phase === 'done') {
      E.actions.innerHTML = `<button class="btn-gold !px-8" data-a="start">${st.phase === 'done' ? 'Nueva mano' : 'Repartir'}</button>`;
    } else if (st.facing) {
      E.actions.innerHTML = `<button class="btn btn-red" data-a="fold">Retirarse</button>
        <button class="btn btn-green" data-a="call" ${Wallet.canAfford(A) ? '' : 'disabled'}>Igualar ${fmt(A)}</button>`;
    } else {
      E.actions.innerHTML = `<button class="btn btn-red" data-a="fold">Retirarse</button>
        <button class="btn" data-a="check">Pasar</button>
        <button class="btn-gold" data-a="raise" ${Wallet.canAfford(A) ? '' : 'disabled'}>Apostar ${fmt(A)}</button>`;
    }
  }

  /* ---------- IA de la casa ---------- */
  function strength() {
    const all = [...st.bot, ...st.board];
    if (all.length < 5) {
      const [a, b] = st.bot.map(c => c.r).sort((x, y) => y - x);
      let s = (a + b) / 28 * 0.45 + (a === b ? 0.3 + a / 60 : 0) + (st.bot[0].s === st.bot[1].s ? 0.05 : 0) + (a - b === 1 ? 0.04 : 0);
      return Math.min(1, s) + rand(-0.08, 0.08);
    }
    const { v } = best(all);
    const base = [0.12, 0.42, 0.68, 0.78, 0.86, 0.9, 0.95, 0.99, 1][v[0]];
    const topPair = v[0] === 1 && v[1] >= 11 ? 0.12 : 0;
    return base + topPair + rand(-0.08, 0.08);
  }

  /* ---------- Flujo ---------- */
  function start() {
    if (!Wallet.debit(st.ante)) return;
    const ante = st.ante;
    const deck = newDeck(1);
    st = { phase: 'play', deck, me: [deck.pop(), deck.pop()], bot: [deck.pop(), deck.pop()], board: [], pot: ante * 2, inv: ante, ante, stage: 0, facing: false, msg: '' };
    log = []; say(`Ante: ${fmt(ante)} cada uno`);
    render();
  }
  function advance() {
    st.stage++;
    if (st.stage > 3) return showdown();
    const n = st.stage === 1 ? 3 : 1;
    for (let i = 0; i < n; i++) st.board.push(st.deck.pop());
    say(`${STAGES[st.stage]} repartido`);
    render();
  }
  function endHand(result, text, ret = 0) {
    Wallet.credit(ret); Wallet.log('Póker', st.inv, ret);
    st.phase = 'done'; st.facing = false; st.msg = text;
    Toast.show(text, result);
    say(text);
  }
  function showdown() {
    const all = best([...st.me, ...st.board]), bot = best([...st.bot, ...st.board]);
    const c = cmp(all.v, bot.v);
    if (c > 0) endHand('win', `¡Ganas con ${HAND_NAMES[all.v[0]]}! +${fmt(st.pot - st.inv)}`, st.pot);
    else if (c < 0) endHand('lose', `La casa gana con ${HAND_NAMES[bot.v[0]]}`);
    else endHand('', 'Empate: recuperas tu apuesta', st.inv);
    render(true, c >= 0 ? all.cards : bot.cards);
  }

  function act(a) {
    const A = st.ante;
    if (a === 'start') return start();
    if (st.phase !== 'play') return;
    if (a === 'fold') { endHand('lose', 'Te retiras'); return render(false); }
    if (a === 'check') {
      say(`${STAGES[st.stage]}: pasas`);
      if (strength() > 0.66 || Math.random() < 0.1) {
        st.pot += A; st.facing = true; say(`La casa apuesta ${fmt(A)}`);
        st.msg = 'La casa apuesta. ¿Igualas?'; return render();
      }
      say('La casa pasa'); st.msg = ''; return advance();
    }
    if (a === 'call') {
      if (!Wallet.debit(A)) return;
      st.pot += A; st.inv += A; st.facing = false; st.msg = ''; say(`Igualas ${fmt(A)}`); return advance();
    }
    if (a === 'raise') {
      if (!Wallet.debit(A)) return;
      st.pot += A; st.inv += A; say(`${STAGES[st.stage]}: apuestas ${fmt(A)}`);
      if (strength() > 0.36) {
        st.pot += A; say(`La casa iguala ${fmt(A)}`); st.msg = ''; return advance();
      }
      say('La casa se retira');
      endHand('win', `La casa se retira. ¡Ganas ${fmt(st.pot - st.inv)}!`, st.pot);
      return render(false);
    }
  }

  root.addEventListener('click', e => { const b = e.target.closest('[data-a]'); if (b) act(b.dataset.a); });
  bindChipTray($('#pk-chips', root), v => { st.ante = v; $('#pk-ante', root).textContent = fmt(v); });
  $('#pk-ante', root).textContent = fmt(st.ante);
  render();
})();
