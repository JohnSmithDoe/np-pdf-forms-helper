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
// Adding a genuinely new FIELD is not one line: it is a Rust enum variant, a
// parser arm, a row here and the mirrored type. Nothing structural, but say so
// rather than pretending otherwise.
// ────────────────────────────────────────────────────────────────

import type { FieldKind, PartnerRolle } from './trains.types';

export type EntityGroup = 'wagen' | 'event' | 'partner' | 'radsatz';

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
