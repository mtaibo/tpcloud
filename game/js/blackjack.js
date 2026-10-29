/* Blackjack: 6 barajas, el crupier se planta en 17, BJ paga 3:2 */
(() => {
  const root = $('#view-blackjack');
  let shoe = [], bet = 10, phase = 'bet', player = [], dealer = [], stake = 0, doubled = false;

  root.innerHTML = `
    <div class="flex items-end justify-between mb-5 flex-wrap gap-2">
      <div>
        <h1 class="font-display text-3xl font-extrabold text-white">Blackjack</h1>
        <p class="text-sm text-slate-400">Blackjack paga 3:2 · El crupier se planta en 17 · Doblar con cualquier mano</p>
      </div>
    </div>
    <div class="felt p-5 sm:p-8">
      <div class="text-center">
        <div class="label-sm !text-emerald-200/70 mb-2">Crupier <span id="bj-dtotal" class="ml-1 px-2 py-0.5 rounded bg-black/40 text-white"></span></div>
        <div id="bj-dealer" class="flex justify-center gap-2 min-h-[100px]"></div>
      </div>
      <div class="my-5 text-center min-h-[56px] grid place-items-center">
        <div id="bj-msg" class="text-xl sm:text-2xl font-display font-extrabold text-gold-light result-banner"></div>
      </div>
      <div class="text-center">
        <div id="bj-player" class="flex justify-center gap-2 min-h-[100px]"></div>
        <div class="label-sm !text-emerald-200/70 mt-2">Tú <span id="bj-ptotal" class="ml-1 px-2 py-0.5 rounded bg-black/40 text-white"></span></div>
      </div>
    </div>

    <div class="card-panel mt-5 p-5 flex flex-wrap items-center justify-between gap-5">
      <div id="bj-bet-ui">
        <div class="label-sm mb-2">Apuesta: <span class="text-gold text-sm tabular-nums" id="bj-bet">10,00</span></div>
        <div class="flex gap-3 items-center" id="bj-chips">${chipTrayHTML([1, 5, 10, 25, 100, 500], 10)}</div>
      </div>
      <div class="flex gap-2 flex-wrap" id="bj-actions"></div>
    </div>`;

  const els = {
    dealer: $('#bj-dealer', root), player: $('#bj-player', root), msg: $('#bj-msg', root),
    dt: $('#bj-dtotal', root), pt: $('#bj-ptotal', root), actions: $('#bj-actions', root),
  };

  const draw = () => { if (shoe.length < 60) shoe = newDeck(6); return shoe.pop(); };
  function val(h) {
    let t = 0, a = 0;
    h.forEach(c => { if (c.r === 14) { a++; t += 11; } else t += Math.min(c.r, 10); });
    while (t > 21 && a) { t -= 10; a--; }
    return { t, soft: a > 0 };
  }
  const isBJ = h => h.length === 2 && val(h).t === 21;

  function renderHands(hideHole) {
    els.dealer.replaceChildren(...dealer.map((c, i) => cardEl(c, hideHole && i === 1)));
    els.player.replaceChildren(...player.map(c => cardEl(c)));
    els.dt.textContent = dealer.length ? (hideHole ? val([dealer[0]]).t : val(dealer).t) : '';
    const p = val(player);
    els.pt.textContent = player.length ? (p.soft && p.t < 21 ? `${p.t - 10} / ${p.t}` : p.t) : '';
  }

  function renderActions() {
    const play = phase === 'play';
    const canDouble = play && player.length === 2 && Wallet.canAfford(bet);
    els.actions.innerHTML = phase === 'bet' || phase === 'done'
      ? `<button class="btn-gold !px-8" id="bj-deal">${phase === 'done' ? 'Nueva mano' : 'Repartir'}</button>`
      : `<button class="btn btn-green" id="bj-hit" ${play ? '' : 'disabled'}>Pedir</button>
         <button class="btn btn-red" id="bj-stand" ${play ? '' : 'disabled'}>Plantarse</button>
         <button class="btn" id="bj-double" ${canDouble ? '' : 'disabled'}>Doblar</button>`;
    $('#bj-bet-ui', root).style.opacity = phase === 'play' || phase === 'dealer' ? .4 : 1;
    $('#bj-bet-ui', root).style.pointerEvents = phase === 'play' || phase === 'dealer' ? 'none' : '';
  }

  async function deal() {
    if (!Wallet.debit(bet)) return;
    stake = bet; doubled = false; phase = 'play';
    els.msg.textContent = '';
    player = []; dealer = []; renderHands(true); renderActions();
    for (const [hand] of [[player], [dealer], [player], [dealer]]) {
      hand.push(draw()); renderHands(true); await sleep(220);
    }
    if (isBJ(player) || isBJ(dealer)) return finish();
    renderActions();
  }

  async function hit() {
    player.push(draw()); renderHands(true);
    if (val(player).t >= 21) return finish();
    renderActions();
  }

  async function dbl() {
    if (!Wallet.debit(bet)) return;
    stake += bet; doubled = true;
    player.push(draw()); renderHands(true);
    await sleep(400);
    finish();
  }

  async function finish() {
    phase = 'dealer'; renderActions(); renderHands(false);
    const p = val(player).t;
    if (p <= 21 && !isBJ(player)) {
      while (val(dealer).t < 17) { await sleep(600); dealer.push(draw()); renderHands(false); }
    }
    const d = val(dealer).t;
    let ret = 0, text, type = '';
    if (p > 21) { text = 'Te pasas de 21'; type = 'lose'; }
    else if (isBJ(player) && !isBJ(dealer)) { ret = stake * 2.5; text = '¡BLACKJACK! Pago 3:2'; type = 'win'; }
    else if (isBJ(dealer) && !isBJ(player)) { text = 'Blackjack del crupier'; type = 'lose'; }
    else if (d > 21) { ret = stake * 2; text = 'El crupier se pasa. ¡Ganas!'; type = 'win'; }
    else if (p > d) { ret = stake * 2; text = '¡Ganas la mano!'; type = 'win'; }
    else if (p === d) { ret = stake; text = 'Empate (push)'; }
    else { text = 'Gana el crupier'; type = 'lose'; }
    Wallet.credit(ret); Wallet.log('Blackjack', stake, ret);
    els.msg.textContent = text + (ret > stake ? ` +${fmt(ret - stake)}` : '');
    els.msg.style.animation = 'none'; void els.msg.offsetWidth; els.msg.style.animation = '';
    if (type) Toast.show(text, type);
    phase = 'done'; renderActions();
  }

  root.addEventListener('click', e => {
    const id = e.target.closest('button')?.id;
    if (id === 'bj-deal' && (phase === 'bet' || phase === 'done')) deal();
    else if (phase === 'play') {
      if (id === 'bj-hit') hit();
      else if (id === 'bj-stand') finish();
      else if (id === 'bj-double') dbl();
    }
  });
  bindChipTray($('#bj-chips', root), v => { bet = v; $('#bj-bet', root).textContent = fmt(v); });
  $('#bj-bet', root).textContent = fmt(bet);
  renderActions();
})();
