// ─── why ────────────────────────────────────────────────────────
// What one Wagen's row says about its Zustand — the columns of the customer's
// dashboard „Alle Wagen Überblick“ that the Schattensystem now holds as records.
// Pure, so `now` is an argument: the dashboard's „Tage seit letztem Mal funken“
// is CALCULATED from the reading's moment, never stored, and a stored number
// would be stale the next morning.
//
// The card's facts only; everything else about a Wagen is its detail page,
// built in Rust (`trains::detail`).
//
// `STUMM_AB_TAGEN` is the dashboard's own threshold (red above 7 days). An open
// Schadensmeldung has no `erledigtAm` — the orange row; an open Auftrag no
// `ausgangAm`. The next Prüfung is the earliest due one per Art that is not done,
// because a P8 and a revision are two deadlines and one hides the other.
//
// A Zeitpunkt is civil: `2026-10-05T08:15:00` is read as local time, the way the
// sender wrote it, so it is never shifted by a zone.
// ────────────────────────────────────────────────────────────────

import type {
  Pruefung,
  Schadensmeldung,
  TelematikMeldung,
  Werkstattauftrag,
  WagenZustand,
} from '../model/trains.types';
import { formatIsoDate } from './uic.util';

export const STUMM_AB_TAGEN = 7;

export interface ZustandIndex {
  meldung: Map<string, TelematikMeldung>;
  schaeden: Map<string, Schadensmeldung[]>;
  auftraege: Map<string, Werkstattauftrag[]>;
  pruefungen: Map<string, Pruefung[]>;
}

export interface ZustandSummary {
  standort: string;
  funk: string;
  stumm: boolean;
  offeneSchaeden: number;
  offeneAuftraege: number;
  naechstePruefung: string;
}

export function indexZustand(zustand: WagenZustand): ZustandIndex {
  return {
    meldung: new Map(zustand.meldungen.map((item) => [item.wagenId, item])),
    schaeden: group(zustand.schaeden),
    auftraege: group(zustand.auftraege),
    pruefungen: group(zustand.pruefungen),
  };
}

export function summarise(
  wagenId: string,
  index: ZustandIndex,
  now: Date
): ZustandSummary {
  const meldung = index.meldung.get(wagenId);
  const offeneSchaeden = (index.schaeden.get(wagenId) ?? []).filter(
    (schaden) => !schaden.erledigtAm
  );
  const offeneAuftraege = (index.auftraege.get(wagenId) ?? []).filter(
    (auftrag) => !auftrag.ausgangAm
  );
  const faellig = naechste(index.pruefungen.get(wagenId) ?? []);
  const erste = faellig.at(0);
  const tage = meldung ? tageSeit(meldung.zeitpunkt, now) : undefined;

  return {
    standort: meldung ? ort(meldung) : '',
    funk: tage === undefined ? '' : funkText(tage),
    stumm: tage !== undefined && tage > STUMM_AB_TAGEN,
    offeneSchaeden: offeneSchaeden.length,
    offeneAuftraege: offeneAuftraege.length,
    naechstePruefung: erste ? pruefungText(erste) : '',
  };
}

function group<T extends { wagenId: string }>(items: T[]): Map<string, T[]> {
  const grouped = new Map<string, T[]>();
  for (const item of items) {
    grouped.set(item.wagenId, [...(grouped.get(item.wagenId) ?? []), item]);
  }
  return grouped;
}

function tageSeit(zeitpunkt: string, now: Date): number | undefined {
  const moment = new Date(zeitpunkt);
  if (Number.isNaN(moment.getTime())) return undefined;
  return Math.max(
    0,
    Math.floor((now.getTime() - moment.getTime()) / 86_400_000)
  );
}

function funkText(tage: number): string {
  if (tage === 0) return 'funkte heute';
  return tage === 1 ? 'funkte vor 1 Tag' : `funkte vor ${tage} Tagen`;
}

function ort(meldung: TelematikMeldung): string {
  const ort = meldung.standort ?? meldung.stadt ?? '';
  return [ort, meldung.land].filter(Boolean).join(', ');
}

function naechste(pruefungen: Pruefung[]): Pruefung[] {
  const offen = pruefungen
    .filter((pruefung) => !pruefung.durchgefuehrtAm && pruefung.faelligAm)
    .sort((left, right) =>
      (left.faelligAm ?? '').localeCompare(right.faelligAm ?? '')
    );
  const seen = new Set<string>();
  return offen.filter((pruefung) => {
    const art = pruefung.art ?? '';
    if (seen.has(art)) return false;
    seen.add(art);
    return true;
  });
}

function pruefungText(pruefung: Pruefung): string {
  const art = pruefung.art ?? 'Prüfung';
  const faellig = pruefung.faelligAm ? formatIsoDate(pruefung.faelligAm) : '';
  return faellig ? `${art} fällig ${faellig}` : art;
}
