// ─── why ────────────────────────────────────────────────────────
// The presentation table over the CLOSED wire enums — `FieldKind` and
// `PartnerRolle`. The wire spellings are ASCII by rule; the labels are where the
// umlauts live, which is why `eigentuemer` is shown as „Eigentümer" and nothing
// renders a raw wire value.
// This is what the mapping screen offers and what auto-mapping matches against.
//
// Only `uic` is required. The wagen number is the only anchor an import needs;
// a date is not, because plenty of the documents that arrive are about something
// other than a dated repair.
//
// `aliases` is the file that will actually grow as real sender files arrive, so
// it stays pure data — a new spelling is one string here and nothing else. They
// are matched as substrings against a normalised header, longest first, so
// `wagennummer` wins over `nummer` on a header that contains both.
//
// The Wagen-Zustand's groups — Telematik, Schadensmeldung, Werkstattauftrag,
// Prüfung — carry deliberately long aliases: a short one (`ort`, `art`) would
// claim a column of another group before the specific one is ever reached.
//
// Adding a genuinely new FIELD is not one line: it is a Rust enum variant, a
// parser arm, a row here and the mirrored type. Nothing structural, but say so
// rather than pretending otherwise.
// ────────────────────────────────────────────────────────────────

import type { FieldKind, PartnerRolle } from './trains.types';

export type EntityGroup =
  | 'wagen'
  | 'event'
  | 'partner'
  | 'radsatz'
  | 'telematik'
  | 'schaden'
  | 'auftrag'
  | 'pruefung';

export interface FieldDescriptor {
  field: FieldKind;
  label: string;
  group: EntityGroup;
  required: boolean;
  aliases: string[];
}

export const FIELD_CATALOGUE: FieldDescriptor[] = [
  {
    field: 'wagennummer',
    label: 'Wagennummer',
    group: 'wagen',
    required: true,
    aliases: [
      'wagennummer',
      'wagen nr',
      'wagen-nr',
      'wagen no',
      'waggon no',
      'wagon',
      'uic',
      'fahrzeugnummer',
      'fahrzeug',
      'evn',
    ],
  },
  {
    field: 'datum',
    label: 'Datum',
    group: 'event',
    required: false,
    aliases: ['datum', 'date', 'leistungsdatum', 'ausfuehrung', 'ausführung'],
  },
  {
    field: 'werkstatt',
    label: 'Werkstatt',
    group: 'partner',
    required: false,
    aliases: ['werkstatt', 'workshop', 'ausfuehrender', 'lieferant', 'betrieb'],
  },
  {
    field: 'halter',
    label: 'Halter',
    group: 'partner',
    required: false,
    aliases: ['halter', 'keeper', 'kunde', 'mieter', 'vermieter', 'einsteller'],
  },
  {
    field: 'eigentuemer',
    label: 'Eigentümer',
    group: 'partner',
    required: false,
    aliases: ['eigentuemer', 'eigentümer', 'owner', 'eigner'],
  },
  {
    field: 'leistung',
    label: 'Leistung',
    group: 'event',
    required: false,
    aliases: [
      'leistung',
      'arbeit',
      'massnahme',
      'maßnahme',
      'taetigkeit',
      'tätigkeit',
      'art',
    ],
  },
  {
    field: 'betrag',
    label: 'Betrag',
    group: 'event',
    required: false,
    aliases: [
      'betrag',
      'kosten',
      'preis',
      'summe',
      'eur',
      'amount',
      'cost',
      'netto',
    ],
  },
  {
    field: 'bemerkung',
    label: 'Bemerkung',
    group: 'event',
    required: false,
    aliases: [
      'bemerkung',
      'beschreibung',
      'notiz',
      'kommentar',
      'description',
      'text',
    ],
  },
  {
    field: 'radsatznummer',
    label: 'Radsatznummer',
    group: 'radsatz',
    required: false,
    aliases: [
      'radsatznummer',
      'radsatz nr',
      'radsatz-nr',
      'rs nr',
      'radsatz',
      'wheelset',
    ],
  },
  {
    field: 'wellennummer',
    label: 'Radsatzwellennummer',
    group: 'radsatz',
    required: false,
    aliases: ['wellennummer', 'radsatzwelle', 'achsnummer', 'wellen nr'],
  },
  {
    field: 'radsatzSystemId',
    label: 'Radsatz-ID',
    group: 'radsatz',
    required: false,
    aliases: ['radsatzid', 'radsatz id', 'radsatz-id'],
  },
  {
    field: 'einbauposition',
    label: 'Einbauposition',
    group: 'radsatz',
    required: false,
    aliases: ['einbauposition', 'einbauort', 'position', 'pos'],
  },
  {
    field: 'eingebautAm',
    label: 'Eingebaut am',
    group: 'radsatz',
    required: false,
    aliases: ['eingebaut am', 'eingebaut', 'einbaudatum', 'einbau', 'montiert'],
  },
  {
    field: 'ausgebautAm',
    label: 'Ausgebaut am',
    group: 'radsatz',
    required: false,
    aliases: [
      'ausgebaut am',
      'ausgebaut',
      'ausbaudatum',
      'ausbau',
      'demontiert',
    ],
  },
  {
    field: 'telematikGeraet',
    label: 'Telematik-Gerät',
    group: 'telematik',
    required: false,
    aliases: ['pointer name', 'pointer', 'geraet', 'gerät', 'telematik id'],
  },
  {
    field: 'telematikAngebautAm',
    label: 'Telematik angebaut am',
    group: 'telematik',
    required: false,
    aliases: ['anbaudatum', 'angebaut am', 'anbau'],
  },
  {
    field: 'telematikZeitpunkt',
    label: 'Telematik-Zeitpunkt',
    group: 'telematik',
    required: false,
    aliases: ['timestamp', 'zeitstempel', 'zeitpunkt', 'letzte meldung'],
  },
  {
    field: 'telematikStadt',
    label: 'Stadt',
    group: 'telematik',
    required: false,
    aliases: ['stadt', 'city'],
  },
  {
    field: 'telematikLand',
    label: 'Land',
    group: 'telematik',
    required: false,
    aliases: ['land', 'country'],
  },
  {
    field: 'telematikStandort',
    label: 'Standort',
    group: 'telematik',
    required: false,
    aliases: ['standort', 'bahnhof', 'location'],
  },
  {
    field: 'telematikLaufleistung',
    label: 'Laufleistung (km)',
    group: 'telematik',
    required: false,
    aliases: [
      'summe laufleistung',
      'laufleistung',
      'kilometer',
      'km-stand',
      'mileage',
    ],
  },
  {
    field: 'telematikEnergie',
    label: 'Energie-Reserve (%)',
    group: 'telematik',
    required: false,
    aliases: ['energie-reserve', 'energie', 'batterie', 'akku'],
  },
  {
    field: 'telematikBewegung',
    label: 'Bewegung',
    group: 'telematik',
    required: false,
    aliases: ['accstatus text', 'bewegung', 'bewegungsstatus'],
  },
  {
    field: 'schadenGemeldetAm',
    label: 'Schaden gemeldet am',
    group: 'schaden',
    required: false,
    aliases: ['gemeldet am', 'meldedatum', 'datum der meldung'],
  },
  {
    field: 'schadenGemeldetVon',
    label: 'Schaden gemeldet von',
    group: 'schaden',
    required: false,
    aliases: ['gemeldet von', 'melder'],
  },
  {
    field: 'schadcode',
    label: 'Schadcode',
    group: 'schaden',
    required: false,
    aliases: ['schadcode', 'schadenscode', 'avv'],
  },
  {
    field: 'schadenNotiz',
    label: 'Schadensnotiz',
    group: 'schaden',
    required: false,
    aliases: ['gemeldeter schaden', 'schadensbeschreibung', 'schaden'],
  },
  {
    field: 'ausgesetzt',
    label: 'Ausgesetzt',
    group: 'schaden',
    required: false,
    aliases: ['ausgesetzt'],
  },
  {
    field: 'beladen',
    label: 'Beladen',
    group: 'schaden',
    required: false,
    aliases: ['beladen', 'beladung'],
  },
  {
    field: 'schadenAusfuehrender',
    label: 'Ausführende Werkstatt/EVU',
    group: 'schaden',
    required: false,
    aliases: ['ausführende werkstatt', 'ausfuehrende werkstatt', 'evu'],
  },
  {
    field: 'schadenGeplantAm',
    label: 'Schaden geplant am',
    group: 'schaden',
    required: false,
    aliases: ['wann'],
  },
  {
    field: 'schadenAktion',
    label: 'Notwendige Aktion',
    group: 'schaden',
    required: false,
    aliases: ['notwendige aktion', 'aktion'],
  },
  {
    field: 'schadenErledigtAm',
    label: 'Schaden erledigt am',
    group: 'schaden',
    required: false,
    aliases: ['erledigt am', 'behoben am', 'erledigt'],
  },
  {
    field: 'bestellnummer',
    label: 'Bestellnummer',
    group: 'auftrag',
    required: false,
    aliases: [
      'bestellnummer',
      'bestellnr',
      'bestell-nr',
      'auftragsnummer',
      'auftragsnr',
    ],
  },
  {
    field: 'auftragStatus',
    label: 'Auftragsstatus',
    group: 'auftrag',
    required: false,
    aliases: ['auftragsstatus'],
  },
  {
    field: 'auftragErfasstAm',
    label: 'Auftrag erfasst am',
    group: 'auftrag',
    required: false,
    aliases: ['best_datum', 'bestelldatum', 'erfasst am', 'erfassungsdatum'],
  },
  {
    field: 'auftragEingangAm',
    label: 'Werkstatteingang',
    group: 'auftrag',
    required: false,
    aliases: [
      'eingang_ist',
      'eingang in werkstatt',
      'werkstatteingang',
      'eingang',
    ],
  },
  {
    field: 'auftragAusgangAm',
    label: 'Werkstattausgang',
    group: 'auftrag',
    required: false,
    aliases: [
      'werk_ausg_ist',
      'ausgang aus werkstatt',
      'werkstattausgang',
      'ausgang',
    ],
  },
  {
    field: 'auftragVersendetAm',
    label: 'Auftrag versendet am',
    group: 'auftrag',
    required: false,
    aliases: ['auftrag versendet', 'versendet am'],
  },
  {
    field: 'auftragBemerkung',
    label: 'Auftragsbemerkung',
    group: 'auftrag',
    required: false,
    aliases: ['bemerkung_intern', 'auftragsbemerkung'],
  },
  {
    field: 'pruefart',
    label: 'Prüfart',
    group: 'pruefung',
    required: false,
    aliases: ['prüfungsart', 'pruefungsart', 'prüfart', 'pruefart', 'rev typ'],
  },
  {
    field: 'pruefungFaelligAm',
    label: 'Prüfung fällig am',
    group: 'pruefung',
    required: false,
    aliases: [
      'fälligkeitstermin',
      'faelligkeitstermin',
      'fälligkeit',
      'faelligkeit',
      'termin',
      'fällig',
    ],
  },
  {
    field: 'pruefungGeplantAm',
    label: 'Prüfung geplant am',
    group: 'pruefung',
    required: false,
    aliases: ['plandatum', 'geplant am'],
  },
  {
    field: 'pruefungDurchgefuehrtAm',
    label: 'Prüfung durchgeführt am',
    group: 'pruefung',
    required: false,
    aliases: [
      'durchgeführt am',
      'durchgefuehrt am',
      'durchgeführt',
      'durchgefuehrt',
    ],
  },
  {
    field: 'pruefungStatus',
    label: 'Prüfungsstatus',
    group: 'pruefung',
    required: false,
    aliases: ['prüfungsstatus', 'pruefungsstatus', 'status'],
  },
  {
    field: 'ignorieren',
    label: 'Nicht importieren',
    group: 'event',
    required: false,
    aliases: [],
  },
];

export const GROUP_LABELS: Record<EntityGroup, string> = {
  wagen: 'Wagen',
  event: 'Instandhaltung',
  partner: 'Partner',
  radsatz: 'Radsatz',
  telematik: 'Telematik',
  schaden: 'Schadensmeldung',
  auftrag: 'Werkstattauftrag',
  pruefung: 'Prüfung',
};

export function labelOf(field: FieldKind): string {
  return FIELD_CATALOGUE.find((entry) => entry.field === field)?.label ?? field;
}

export function requiredFields(): FieldKind[] {
  return FIELD_CATALOGUE.filter((entry) => entry.required).map(
    (entry) => entry.field
  );
}

export const ROLLEN_LABELS: Record<PartnerRolle, string> = {
  halter: 'Halter',
  eigentuemer: 'Eigentümer',
  werkstatt: 'Werkstatt',
};

export function rollenLabel(rollen: PartnerRolle[]): string {
  return rollen.map((rolle) => ROLLEN_LABELS[rolle]).join(', ');
}
