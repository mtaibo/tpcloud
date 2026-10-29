/* Chicken Road: cruza la carretera carril a carril; cada cruce multiplica, pero un coche puede atropellarte */
(() => {
  const root = $('#view-chicken');
  const LANES = 15, EDGE = 0.97;
  const LEVELS = {
    easy: { name: 'Fácil', p: 0.92 }, medium: { name: 'Medio', p: 0.85 },
    hard: { name: 'Difícil', p: 0.75 }, extreme: { name: 'Extremo', p: 0.6 },
  };
  const CARS = ['🚗', '🚕', '🚙', '🚌', '🚚', '🏎️'];
  let level = 'medium', bet = 0, pos = 0, active = false, busy = false;

  const mult = k => Math.floor(EDGE / Math.pow(LEVELS[level].p, k) * 100) / 100;

  root.innerHTML = gameHeader('Chicken Road', 'Ayuda a la gallina a cruzar la carretera. ¿Hasta dónde te atreves?', '97 %') + `
    <div class="grid lg:grid-cols-[330px_1fr] gap-5">
      <div class="card-panel p-5 space-y-4">
        ${betBoxHTML('ck-bet', 10)}
        <div>
          <div class="label-sm mb-2">Dificultad</div>
          <div class="grid grid-cols-2 gap-1" id="ck-levels">
            ${Object.entries(LEVELS).map(([k, v]) => `<button class="tab !rounded-lg ${k === level ? 'active' : ''}" data-l="${k}">${v.name}<br><small class="opacity-70">${Math.round(v.p * 100)} % por carril</small></button>`).join('')}
          </div>
        </div>
        <button id="ck-start" class="btn-gold w-full !py-3 text-base">Apostar</button>
        <button id="ck-go" class="btn btn-green w-full !py-3 text-base" disabled>🐔 Cruzar</button>
        <button id="ck-cash" class="btn w-full !py-3" disabled>Retirar</button>
        <dl class="text-sm space-y-1.5 border-t border-line pt-4">
          <div class="flex justify-between"><dt class="text-slate-400">Carril</dt><dd class="font-bold text-white" id="ck-lane">0 / ${LANES}</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Multiplicador</dt><dd class="font-bold text-white" id="ck-mult">1,00×</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Si cruzas</dt><dd class="font-bold text-slate-300" id="ck-next">–</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Beneficio</dt><dd class="font-bold text-gold" id="ck-profit">0,00</dd></div>
        </dl>
      </div>
      <div class="card-panel p-3 sm:p-4 min-w-0">
        <div class="road-wrap" id="ck-wrap"><div class="road" id="ck-road"></div></div>
        <p class="text-xs text-slate-500 mt-3 px-1" id="ck-hint">Elige dificultad y pulsa Apostar para empezar.</p>
      </div>
    </div>`;

  const road = $('#ck-road', root), wrap = $('#ck-wrap', root);

  function buildRoad() {
    road.innerHTML = `<div class="lane-start"></div>` +
      Array.from({ length: LANES }, (_, i) => `
        <div class="lane-col" data-lane="${i + 1}">
          <div class="lane-mult">${fmt(mult(i + 1))}×</div>
          <div class="lane-road"><div class="lane-manhole"></div></div>
        </div>`).join('') + `<div class="lane-end"><span>🏁</span></div><div class="chicken" id="ck-chicken">🐔</div>`;
    placeChicken(0, false);
  }

  function placeChicken(k, animate = true) {
    const c = $('#ck-chicken', root);
    if (k === 0) { c.style.left = '36px'; c.style.top = '58%'; wrap.scrollTo({ left: 0 }); return; }
    const lane = $(`[data-lane="${k}"]`, road);
    c.style.left = (lane.offsetLeft + lane.offsetWidth / 2) + 'px';
    c.style.top = '58%';
    wrap.scrollTo({ left: Math.max(0, lane.offsetLeft - 160), behavior: animate ? 'smooth' : 'auto' });
  }
  function refresh() {
    const m = pos ? mult(pos) : 1;
    $('#ck-lane', root).textContent = `${pos} / ${LANES}`;
    $('#ck-mult', root).textContent = fmt(m) + '×';
    $('#ck-next', root).textContent = active && pos < LANES ? fmt(mult(pos + 1)) + '×' : '–';
    $('#ck-profit', root).textContent = active ? fmt(bet * m - bet) : '0,00';
    $('#ck-go', root).disabled = !active || busy;
    $('#ck-cash', root).disabled = !active || busy || pos === 0;
    $('#ck-cash', root).textContent = active && pos ? `Retirar ${fmt(bet * m)}` : 'Retirar';
    $$('.lane-col', road).forEach(l => {
      const n = +l.dataset.lane;
      l.classList.toggle('done', n <= pos); l.classList.toggle('next', active && n === pos + 1);
    });
  }

  function setLocked(l) {
    setDisabled([$('#ck-bet', root), $('#ck-start', root), ...$$('[data-l],[data-bet-op]', root)], l);
  }

  function start() {
    const b = readBet('ck-bet'); if (!b || !Wallet.debit(b)) return;
    bet = b; pos = 0; active = true; busy = false;
    buildRoad();
    $('#ck-hint', root).textContent = 'Pulsa «Cruzar» para avanzar al siguiente carril.';
    setLocked(true); Sound.play('click'); refresh();
  }

  async function go() {
    if (!active || busy) return;
    busy = true; refresh();
    const next = pos + 1, lane = $(`[data-lane="${next}"]`, road), survive = rnd() < LEVELS[level].p;
    const car = document.createElement('div');
    car.className = 'car'; car.textContent = pick(CARS);
    lane.querySelector('.lane-road').appendChild(car);

    if (survive) {
      // pasa un coche, luego cruza la gallina
      car.style.animation = 'carDrive .8s linear forwards';
      await sleep(850); car.remove();
      pos = next; placeChicken(pos); Sound.play('gem');
      await sleep(350);
      busy = false;
      if (pos === LANES) return cashOut();
      refresh();
    } else {
      // la gallina entra y el coche la atropella
      pos = next; placeChicken(pos); await sleep(300);
      car.style.animation = 'carHit .45s ease-in forwards';
      await sleep(380);
      const ch = $('#ck-chicken', root); ch.textContent = '💥'; ch.classList.add('dead'); Sound.play('boom');
      await sleep(500);
      active = false; busy = false;
      Wallet.log('Chicken Road', bet, 0);
      Toast.show(`¡Atropellada en el carril ${next}! -${fmt(bet)} cr`, 'lose');
      $('#ck-hint', root).textContent = `Te atropellaron en el carril ${next}. Prueba otra vez.`;
      setLocked(false); refresh();
    }
  }

  function cashOut() {
    if (!active || pos === 0) return;
    const m = mult(pos), ret = bet * m;
    active = false; busy = false;
    Wallet.credit(ret); Wallet.log('Chicken Road', bet, ret);
    Sound.play(m >= 10 ? 'big' : 'cash');
    Toast.show(`Retiras ${fmt(ret)} cr (${fmt(m)}×)`, 'win');
    $('#ck-hint', root).textContent = `¡Cobras ${fmt(ret)} cr en el carril ${pos}!`;
    setLocked(false); refresh();
  }

  $('#ck-start', root).onclick = start;
  $('#ck-go', root).onclick = go;
  $('#ck-cash', root).onclick = cashOut;
  $('#ck-levels', root).addEventListener('click', e => {
    const b = e.target.closest('[data-l]'); if (!b || active) return;
    level = b.dataset.l; $$('[data-l]', root).forEach(x => x.classList.toggle('active', x === b));
    buildRoad(); refresh();
  });
  document.addEventListener('keydown', e => {
    if (root.classList.contains('hidden') || e.target.tagName === 'INPUT') return;
    if (e.code === 'Space') { e.preventDefault(); active ? go() : null; }
  });

  buildRoad(); refresh();
})();
