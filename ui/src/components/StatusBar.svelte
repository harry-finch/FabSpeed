<script lang="ts">
  // Barre d'état : connexion, versions, limites de la machine active.
  import { store } from '../lib/store.svelte';
  import { fmtSpeed, num, speedUnit } from '../lib/format';
  import { rpmUnit } from '../lib/labels';
  import { t } from '../lib/i18n.svelte';

  const m = $derived(store.machine);
</script>

<footer class="status">
  <span class="dot" class:off={!store.online}></span>
  <span>{store.online ? t('status.online') : t('status.offline')}</span>
  {#if store.catalog}
    <span class="sep"></span>
    <span>{t('status.engine', { v: store.catalog.engine_version })}</span>
    <span class="sep"></span>
    <span>{t('status.data', { v: store.catalog.meta.revision })}</span>
  {/if}
  {#if m}
    <span class="grow"></span>
    <span>{m.name} · {num(m.rpm_min, 0)}–{num(m.rpm_max, 0)} {rpmUnit()} · {num(m.spindle_power_w / 1000, 1)} kW · {t('status.maxFeed')} {fmtSpeed(m.max_feed_mm_min, store.units)} {speedUnit(store.units)}</span>
  {/if}
</footer>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 24px;
    padding: 0 var(--s3);
    background: var(--bg-chrome);
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: var(--fs-xs);
    white-space: nowrap;
    overflow: hidden;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--success);
  }

  .dot.off {
    background: var(--danger);
  }

  .sep {
    width: 1px;
    height: 12px;
    background: var(--border-strong);
  }

  .grow {
    flex: 1;
  }
</style>
