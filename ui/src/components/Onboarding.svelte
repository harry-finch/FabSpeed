<script lang="ts">
  // Premier lancement guidé : langue (et thème), mot de passe administrateur
  // (facultatif), machine du poste, puis un premier calcul commenté sur les
  // valeurs réelles. Proposé d'office sur un poste neuf, revu depuis l'aide.
  import { Check, ChevronLeft, ChevronRight, Cpu, Plus, ShieldCheck, Monitor, Sun, Moon, FlaskConical } from '@lucide/svelte';
  import { store } from '../lib/store.svelte';
  import { i18n, LOCALES, t } from '../lib/i18n.svelte';
  import { theme, THEMES } from '../lib/theme.svelte';
  import { fmtLength, fmtSpeed, lengthUnit, num, speedUnit } from '../lib/format';
  import { rpmUnit } from '../lib/labels';
  import OriginTag from './OriginTag.svelte';

  const MIN_LENGTH = 8; // identique à MIN_PASSWORD_LEN côté serveur
  const STEPS = ['welcome', 'password', 'machine', 'result'] as const;

  let dialog: HTMLDialogElement;
  let step = $state(0);
  let password = $state('');
  let confirm = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);
  let newMachine = $state(false);

  const u = $derived(store.units);
  const reco = $derived(store.reco);
  const v = $derived(reco?.values);
  const passwordOk = $derived(password.length >= MIN_LENGTH && password === confirm);

  $effect(() => {
    dialog.showModal();
  });

  async function setPassword() {
    if (!passwordOk || busy) return;
    busy = true;
    error = await store.setupPassword(password);
    busy = false;
    if (!error) step++;
  }

  function finish() {
    store.finishOnboarding();
    if (newMachine) store.openLibrary('machines', null, true);
    else store.module = 'calc';
    dialog.close();
  }
</script>

<dialog
  bind:this={dialog}
  onclose={() => store.onboarding && store.finishOnboarding()}
  aria-labelledby="onb-title"
>
  <header>
    <img src="/fabspeed-logo.svg?v=2" alt="" width="28" height="28" />
    <h2 id="onb-title">{t(`onboarding.${STEPS[step]}.title`)}</h2>
    <ol class="dots" aria-label={t('onboarding.progress', { n: step + 1, total: STEPS.length })}>
      {#each STEPS as s, i (s)}<li class:on={i === step} class:done={i < step}></li>{/each}
    </ol>
  </header>

  <div class="body">
    {#if STEPS[step] === 'welcome'}
      <p class="lead">{t('onboarding.welcome.lead')}</p>
      <p>{t('onboarding.welcome.text')}</p>
      <div class="choices">
        <div>
          <span class="lbl">{t('app.language')}</span>
          <div class="seg" role="radiogroup" aria-label={t('app.language')}>
            {#each LOCALES as l (l.id)}
              <button type="button" role="radio" aria-checked={i18n.locale === l.id} class:on={i18n.locale === l.id} onclick={() => store.setLocale(l.id)}>{l.label}</button>
            {/each}
          </div>
        </div>
        <div>
          <span class="lbl">{t('app.theme')}</span>
          <div class="seg" role="radiogroup" aria-label={t('app.theme')}>
            {#each THEMES as th (th)}
              {@const Icon = { system: Monitor, light: Sun, dark: Moon }[th]}
              <button type="button" role="radio" aria-checked={theme.choice === th} class:on={theme.choice === th} onclick={() => theme.set(th)}>
                <Icon size={13} /> {t(`onboarding.themes.${th}`)}
              </button>
            {/each}
          </div>
        </div>
      </div>
    {:else if STEPS[step] === 'password'}
      <p>{t('onboarding.password.text')}</p>
      {#if store.auth.configured}
        <p class="ok"><ShieldCheck size={15} /> {t('onboarding.password.already')}</p>
      {:else}
        <form
          class="pw"
          onsubmit={(e) => {
            e.preventDefault();
            setPassword();
          }}
        >
          <label><span>{t('auth.newPassword')}</span><input class="input" type="password" autocomplete="new-password" bind:value={password} /></label>
          <label><span>{t('auth.confirm')}</span><input class="input" class:bad={confirm !== '' && confirm !== password} type="password" autocomplete="new-password" bind:value={confirm} /></label>
          <p class="hint">{confirm !== '' && confirm !== password ? t('auth.mismatch') : t('auth.minLength', { n: MIN_LENGTH })}</p>
          {#if error}<p class="error" role="alert">{error}</p>{/if}
          <button class="btn primary" type="submit" disabled={!passwordOk || busy}><ShieldCheck size={14} /> {t('onboarding.password.set')}</button>
        </form>
        <p class="hint">{t('onboarding.password.later')}</p>
      {/if}
    {:else if STEPS[step] === 'machine'}
      <p>{t('onboarding.machine.text')}</p>
      <div class="machines" role="radiogroup" aria-label={t('app.machine')}>
        {#each store.catalog?.machines ?? [] as m (m.id)}
          <button
            type="button"
            role="radio"
            aria-checked={!newMachine && store.setup.machineId === m.id}
            class="machine"
            class:on={!newMachine && store.setup.machineId === m.id}
            onclick={() => {
              newMachine = false;
              store.setMachine(m.id);
            }}
          >
            <Cpu size={16} />
            <span class="txt">
              <span class="name">{m.name}</span>
              <span class="sub">{m.category} · {num(m.rpm_min, 0)}–{num(m.rpm_max, 0)} {rpmUnit()} · {num(m.spindle_power_w / 1000, 1)} kW</span>
            </span>
            {#if !newMachine && store.setup.machineId === m.id}<Check size={15} />{/if}
          </button>
        {/each}
        <button type="button" role="radio" aria-checked={newMachine} class="machine" class:on={newMachine} onclick={() => (newMachine = true)}>
          <Plus size={16} />
          <span class="txt">
            <span class="name">{t('onboarding.machine.new')}</span>
            <span class="sub">{t('onboarding.machine.newHint')}</span>
          </span>
          {#if newMachine}<Check size={15} />{/if}
        </button>
      </div>
    {:else}
      <p>{t('onboarding.result.text', { material: store.material?.name ?? '—', tool: store.tool?.reference || store.tool?.name || '—' })}</p>
      {#if v}
        <dl class="values tabular">
          <div><dt>{t('readout.rpm')}</dt><dd>{num(v.rpm, 0)} {rpmUnit()}</dd><p>{t('onboarding.result.rpm')}</p></div>
          <div><dt>{t('readout.feed')}</dt><dd>{fmtSpeed(v.feed_mm_min, u)} {speedUnit(u)}</dd><p>{t('onboarding.result.feed')}</p></div>
          <div><dt>{t('readout.plunge')}</dt><dd>{fmtSpeed(v.plunge_mm_min, u)} {speedUnit(u)}</dd><p>{t('onboarding.result.plunge')}</p></div>
          <div><dt>{t('readout.doc')}</dt><dd>{fmtLength(v.doc_mm, u)} {lengthUnit(u)}</dd><p>{t('onboarding.result.doc')}</p></div>
        </dl>
      {/if}
      <ul class="tips">
        <li><OriginTag origin="calculated" /> {t('onboarding.result.calculated')}</li>
        <li><FlaskConical size={13} /> {t('onboarding.result.record')}</li>
        <li><ShieldCheck size={13} /> {t('onboarding.result.safety')}</li>
      </ul>
    {/if}
  </div>

  <footer>
    {#if step === 0}
      <button class="btn quiet" type="button" onclick={() => dialog.close()}>{t('onboarding.skip')}</button>
    {:else}
      <button class="btn quiet" type="button" onclick={() => step--}><ChevronLeft size={14} /> {t('onboarding.back')}</button>
    {/if}
    <span class="grow"></span>
    {#if step < STEPS.length - 1}
      <button class="btn" class:primary={STEPS[step] !== 'password' || store.auth.configured} type="button" onclick={() => step++}>
        {STEPS[step] === 'password' && !store.auth.configured ? t('onboarding.password.skip') : t('onboarding.next')}
        <ChevronRight size={14} />
      </button>
    {:else}
      <button class="btn primary" type="button" onclick={finish}>
        {newMachine ? t('onboarding.createMachine') : t('onboarding.start')}
      </button>
    {/if}
  </footer>
</dialog>

<style>
  dialog {
    width: min(620px, 94vw);
    padding: 0;
    border: 0;
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: var(--shadow-pop);
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s3) var(--s4);
    border-bottom: 1px solid var(--border);
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: var(--fs-lg);
  }

  .dots {
    display: flex;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .dots li {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border-strong);
  }

  .dots li.done {
    background: var(--text-muted);
  }

  .dots li.on {
    background: var(--accent);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    min-height: 300px;
    max-height: 70vh;
    overflow: auto;
    padding: var(--s4);
    font-size: var(--fs-md);
    line-height: 1.5;
  }

  .body p {
    margin: 0;
    color: var(--text-secondary);
  }

  .body p.lead {
    color: var(--text-primary);
    font-size: var(--fs-lg);
  }

  .choices {
    display: grid;
    gap: var(--s3);
    margin-top: var(--s2);
  }

  .lbl {
    display: block;
    margin-bottom: 4px;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .seg {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background: var(--segment-track);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 var(--s3);
    border: 0;
    border-radius: 3px;
    background: none;
    color: var(--text-secondary);
  }

  .seg button.on {
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .pw {
    display: grid;
    gap: var(--s2);
    max-width: 320px;
  }

  .pw label {
    display: grid;
    gap: 4px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }

  .pw .btn {
    justify-self: start;
  }

  .bad {
    border-color: var(--danger) !important;
  }

  .hint {
    font-size: var(--fs-xs);
    color: var(--text-muted) !important;
  }

  .error {
    color: var(--danger) !important;
  }

  .ok {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--success) !important;
    font-weight: 600;
  }

  .machines {
    display: grid;
    gap: var(--s1);
  }

  .machine {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s2) var(--s3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-panel);
    color: var(--text-secondary);
    text-align: left;
  }

  .machine:hover {
    border-color: var(--border-strong);
  }

  .machine.on {
    border-color: var(--accent);
    background: var(--bg-selected);
    color: var(--text-primary);
  }

  .txt {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .name {
    color: var(--text-primary);
    font-weight: 600;
  }

  .sub {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }

  .values {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--s2) var(--s4);
    margin: 0;
  }

  .values div {
    padding: var(--s2) var(--s3);
    border-radius: var(--radius);
    background: var(--bg-panel);
  }

  .values dt {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .values dd {
    margin: 2px 0 4px;
    font-size: 18px;
    font-weight: 600;
  }

  .values p {
    font-size: var(--fs-sm);
  }

  .tips {
    display: grid;
    gap: var(--s2);
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--text-secondary);
  }

  .tips li {
    display: flex;
    align-items: flex-start;
    gap: var(--s2);
  }

  .tips :global(svg) {
    flex: none;
    margin-top: 3px;
  }

  footer {
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: var(--s3) var(--s4);
    border-top: 1px solid var(--border);
  }

  .grow {
    flex: 1;
  }
</style>
