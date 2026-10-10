// ─── why ────────────────────────────────────────────────────────
// „Alles importieren“: the whole master in one run, nothing asked, a report
// afterwards. The walk asks per sheet and per entity; on a recurring mirror of
// the customer's own file that is hundreds of clicks whose answer is always
// „alle neuen anlegen“. So this answers the way that click does — and only
// that way:
//
//   Known      its match
//   New        created
//   Likely     NOT taken, and named in the report. A near match is a guess,
//   Ambiguous  and a Radsatznummer may never decide alone (fachdomaene.md);
//   Missing    the reference stays empty instead, which a later walk can fill.
//
// A disputed Einbau date keeps the stored one — the walk's default, and the
// answer that writes nothing. Duplicates and rejected rows are left out, as the
// walk starts them.
//
// It is the SAME run as the walk, not a second import: `mirror::start` empties
// the facts and records the run, each sheet goes through `mirror::stage_sheet`
// and `commit::commit` with every gate in it, and `mirror::done` ticks it off,
// in the order `start` chose — the stock must meet the fittings already
// written. A sheet that fails is a report line and the run goes on; it stays
// open in `import_run`, so the page's „unvollständig“ banner names it.
//
// A sheet's commit lines are capped at `LIMIT`: one bad column in a sheet of
// 1,700 rows would otherwise be 1,700 lines nobody reads.
// ────────────────────────────────────────────────────────────────

use super::mirror;
use crate::error::AppResult;
use crate::model::ClientReport;
use crate::trains::commit;
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    EntityChoice, EntityDecision, EntityDecisions, EntityGroup, Resolution, RowStatus, StagedImport,
};
use crate::trains::{entities, farbe};

const LIMIT: usize = 10;

pub fn run(db: &mut TrainsDb, today: &str) -> AppResult<ClientReport> {
    let run = mirror::start(db, today)?;
    let mut messages = Vec::new();
    let mut failed = 0;
    for sheet in &run.sheets {
        match one(db, sheet) {
            Ok(lines) => messages.extend(lines),
            Err(error) => {
                failed += 1;
                messages.push(format!(
                    "„{sheet}“: nicht importiert — {}",
                    error.into_messages().join(" ")
                ));
            }
        }
    }
    let headline = match failed {
        0 => "Master-Datei vollständig importiert".to_string(),
        1 => "Master-Datei importiert, 1 Blatt fehlt".to_string(),
        failed => format!("Master-Datei importiert, {failed} Blätter fehlen"),
    };
    Ok(ClientReport {
        headline,
        messages,
        message_folder: None,
    })
}

fn one(db: &mut TrainsDb, sheet: &str) -> AppResult<Vec<String>> {
    let held = mirror::stage_sheet(db, sheet)?;
    let (decisions, offen) = answers(&held.wire);
    let rows = entities::expand(&held.wire, &decisions);
    let committed = commit::commit(db, &held.wire, &held.values, &rows)?;
    farbe::merge_master(db, &held.farben)?;
    mirror::done(db, sheet)?;

    let mut lines: Vec<String> = Vec::new();
    let mut messages = committed.messages.into_iter();
    if let Some(summary) = messages.next() {
        lines.push(format!("„{sheet}“: {summary}"));
    }
    let rest: Vec<String> = messages.collect();
    lines.extend(capped(sheet, rest));
    if !offen.is_empty() {
        lines.push(format!(
            "„{sheet}“: nicht zugeordnet, weil nicht eindeutig: {}",
            listed(&offen)
        ));
    }
    Ok(lines)
}

fn answers(staging: &StagedImport) -> (EntityDecisions, Vec<String>) {
    let groups = staging.entities.clone().unwrap_or_default();
    let mut offen = Vec::new();
    let mut choose = |groups: &[EntityGroup]| -> Vec<EntityChoice> {
        groups
            .iter()
            .map(|group| {
                let decision = match &group.resolution {
                    Resolution::Known { id, .. } => EntityDecision::Use { id: id.clone() },
                    Resolution::New { .. } => EntityDecision::Create,
                    Resolution::Likely { .. } | Resolution::Ambiguous { .. } => {
                        offen.push(group.spellings.first().cloned().unwrap_or_default());
                        EntityDecision::Skip
                    }
                    Resolution::Missing => EntityDecision::Skip,
                };
                EntityChoice {
                    key: group.key.clone(),
                    decision,
                }
            })
            .collect()
    };
    let decisions = EntityDecisions {
        staging_id: staging.id.clone(),
        partner: choose(&groups.partner),
        wagen: choose(&groups.wagen),
        radsaetze: choose(&groups.radsaetze),
        einbauten: Vec::new(),
        rows: staging
            .rows
            .iter()
            .filter(|row| !matches!(row.status, RowStatus::Rejected | RowStatus::Duplicate))
            .map(|row| row.row)
            .collect(),
    };
    offen.retain(|name| !name.is_empty());
    (decisions, offen)
}

fn capped(sheet: &str, lines: Vec<String>) -> Vec<String> {
    let total = lines.len();
    let mut kept: Vec<String> = lines
        .into_iter()
        .take(LIMIT)
        .map(|line| format!("„{sheet}“: {line}"))
        .collect();
    if total > LIMIT {
        kept.push(format!("„{sheet}“: … und {} weitere", total - LIMIT));
    }
    kept
}

fn listed(names: &[String]) -> String {
    let shown = names
        .iter()
        .take(LIMIT)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if names.len() > LIMIT {
        format!("{shown} … und {} weitere", names.len() - LIMIT)
    } else {
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::master::mirror::tests::bound;

    #[test]
    fn the_whole_master_comes_in_without_a_question_and_the_run_is_complete() {
        let folder = TempDir::new("import-all");
        let mut db = bound(&folder);

        let report = run(&mut db, "2026-10-08").unwrap();

        assert_eq!(report.headline, "Master-Datei vollständig importiert");
        assert_eq!(db.counts().wagen, 2);
        assert_eq!(
            db.counts().radsaetze,
            3,
            "RS3 only the stock knows comes too"
        );
        let run = db.master().import_run.clone().unwrap();
        assert_eq!(run.done, run.sheets, "every sheet ticked off");
        for sheet in ["Übersicht", "Einbauliste", "Bestand"] {
            assert!(
                report
                    .messages
                    .iter()
                    .any(|line| line.starts_with(&format!("„{sheet}“: "))),
                "a line for {sheet}: {:?}",
                report.messages
            );
        }
    }

    #[test]
    fn a_disputed_date_keeps_the_stored_one() {
        let folder = TempDir::new("import-all-keep");
        let mut db = bound(&folder);
        run(&mut db, "2026-10-08").unwrap();

        let id = db.radsaetze_by_key("RS1")[0].id.clone();
        let open: Vec<_> = db
            .einbauten()
            .into_iter()
            .filter(|einbau| einbau.radsatz_id == id && einbau.is_open())
            .collect();
        assert_eq!(open.len(), 1, "never a second fitting or a phantom removal");
        assert_eq!(open[0].source.sheet, "Einbauliste");
    }

    #[test]
    fn a_second_run_rebuilds_rather_than_adds() {
        let folder = TempDir::new("import-all-twice");
        let mut db = bound(&folder);
        run(&mut db, "2026-10-08").unwrap();
        let first = db.counts();
        run(&mut db, "2026-10-09").unwrap();
        assert_eq!(db.counts().wagen, first.wagen);
        assert_eq!(db.counts().radsaetze, first.radsaetze);
    }

    #[test]
    fn a_long_list_of_lines_is_capped_with_the_rest_counted() {
        let lines: Vec<String> = (1..=12).map(|row| format!("Zeile {row}: kaputt")).collect();
        let kept = capped("Bestand", lines);
        assert_eq!(kept.len(), LIMIT + 1);
        assert_eq!(kept[0], "„Bestand“: Zeile 1: kaputt");
        assert_eq!(kept[LIMIT], "„Bestand“: … und 2 weitere");
    }

    #[test]
    fn a_near_match_is_not_taken_and_is_named() {
        let folder = TempDir::new("import-all-likely");
        let mut db = bound(&folder);
        mirror::start(&mut db, "2026-10-08").unwrap();
        let mut held = mirror::stage_sheet(&db, "Übersicht").unwrap();
        let groups = held.wire.entities.as_mut().unwrap();
        groups.wagen[0].resolution = Resolution::Likely {
            id: "w-other".into(),
            name: "anderer Wagen".into(),
            hint: "ähnlich".into(),
        };
        let spelling = groups.wagen[0].spellings[0].clone();

        let (decisions, offen) = answers(&held.wire);

        assert_eq!(decisions.wagen[0].decision, EntityDecision::Skip);
        assert_eq!(decisions.wagen[1].decision, EntityDecision::Create);
        assert_eq!(offen, [spelling]);
    }

    #[test]
    fn a_long_name_list_is_capped_too() {
        let names: Vec<String> = (1..=12).map(|n| format!("RS{n}")).collect();
        assert!(listed(&names).ends_with("RS10 … und 2 weitere"));
    }
}
