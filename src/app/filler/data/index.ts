// ─── why ────────────────────────────────────────────────────────
// The one barrel this app allows. Everywhere else Sheriff runs barrel-less, so
// a file is reached by its path and there is nothing to keep in sync. Here the
// barrel is the seal: it publishes the facade and hides `FillerStore`, so no
// component can reach past it into `@ngrx/signals`. Deep imports into this
// folder are a Sheriff error, which is the enforcement.
// ────────────────────────────────────────────────────────────────

export { FillerFacade } from './filler.facade';
