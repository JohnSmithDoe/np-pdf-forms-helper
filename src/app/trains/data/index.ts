// ─── why ────────────────────────────────────────────────────────
// The domain's seal. Everything outside `trains/data` sees these and nothing
// else — not the stores, not the backend, not `@ngrx/signals`. That is what lets
// the state library be swapped or a store split without a template changing.
//
// The four list facades are part of the surface because a page provides one to
// `LIST_FACADE`; the stores behind them are not.
// ────────────────────────────────────────────────────────────────

export { TrainsFacade } from './trains.facade';
export { ImportFacade } from './import.facade';
export { IntakeFacade } from './intake.facade';
export { ImportWalkFacade } from './import-walk.facade';
export { MasterExportFacade } from './master-export.facade';
export { MasterFileFacade } from './master-file.facade';
export { WagenListFacade } from './wagen-list.facade';
export { PartnerListFacade } from './partner-list.facade';
export { EventListFacade } from './event-list.facade';
export { TemplateListFacade } from './template-list.facade';
export { RadsatzListFacade } from './radsatz-list.facade';
export type { WagenRow } from './wagen-list.facade';
export type { PartnerRow } from './partner-list.facade';
export type { EventRow } from './event-list.facade';
export type { TemplateRow } from './template-list.facade';
export type { RadsatzRow } from './radsatz-list.facade';
export type { Stage } from './import.store';
export type {
  IntakeRow,
  IntakeStep,
  PickOption,
  SummaryRow,
} from './intake.facade';
export type { FileOutcome, FileResult } from './intake.store';
export type {
  BulkAnswer,
  BulkCounts,
  EinbauView,
  EntryView,
  GroupView,
  PlanCount,
} from './import-walk.facade';
export type {
  ColumnAnswer,
  PairView,
  ExportSheetView,
  StructureView,
} from './master-export.facade';
