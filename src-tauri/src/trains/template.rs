// ─── why ────────────────────────────────────────────────────────
// The two ways a template is written, both outside the import itself.
//
// `learned` is what a file's COMPLETED cleaning teaches: the readings the user
// confirmed, saved onto the template the file came through, so the sender's
// next file asks nothing. It runs when the cleaned copy is filed and never
// earlier — a reading confirmed on a file the user then abandoned was confirmed
// about nothing. It used to run at import, which meant a user who only cleans
// answered the same card for every file. It RETURNS the template rather than
// storing it, so the caller writes it in the same transaction as the document:
// a filed document and its lesson are one fact.
//
// A SHIPPED template is never changed — the readings go onto a user copy whose
// `origin` names it, created on first use and updated after that, which then
// shadows the shipped one. Only the readings move: the template's own columns
// and headers are kept, because the plan the user confirmed belongs to one file
// and the template describes them all.
//
// `save` is the mapper's only exit. An unknown file is not imported by hand any
// more; it is mapped, saved as a template, and then cleaned like every other
// file, so nothing reaches the Schattensystem without passing the cleaning.
// That is why a name is REQUIRED here where the old commit made it optional.
// ────────────────────────────────────────────────────────────────

use uuid::Uuid;

use super::clock;
use super::db::TrainsDb;
use super::model::{FieldKind, ImportPlan, ImportTemplate};
use super::recognise;
use crate::error::{AppError, AppResult};

pub fn learned(
    db: &TrainsDb,
    template_id: &str,
    confirmed: &ImportPlan,
) -> Option<(ImportTemplate, String)> {
    let template = db.template(template_id)?;
    let base = if template.builtin {
        db.user_copy_of(template_id)
            .cloned()
            .unwrap_or_else(|| ImportTemplate {
                id: Uuid::new_v4().to_string(),
                name: template.name.clone(),
                plan: template.plan.clone(),
                partner_id: None,
                origin: Some(template.id.clone()),
                builtin: false,
                created_at: clock::today_iso(),
            })
    } else {
        template
    };

    let mut learned = base.clone();
    for binding in &mut learned.plan.columns {
        if binding.field == FieldKind::Ignorieren {
            continue;
        }
        let header = recognise::normalise(&binding.header);
        if let Some(found) = confirmed.columns.iter().find(|column| {
            column.field == binding.field && recognise::normalise(&column.header) == header
        }) {
            binding.decimal = found.decimal.or(binding.decimal);
            binding.date_order = found.date_order.or(binding.date_order);
        }
    }
    if learned.plan == base.plan {
        return None;
    }
    let line = format!(
        "Vorlage „{}“ merkt sich die bestätigten Lesarten.",
        learned.name
    );
    Some((learned, line))
}

pub fn save(name: &str, plan: &ImportPlan) -> AppResult<ImportTemplate> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Report(vec![
            "Die Vorlage braucht einen Namen.".into()
        ]));
    }
    let missing = plan.missing_required();
    if !missing.is_empty() {
        let labels: Vec<&str> = missing.iter().map(|field| field.label()).collect();
        return Err(AppError::Report(vec![
            "Es fehlen noch Pflichtzuordnungen.".into(),
            format!("Bitte ordne zu: {}.", labels.join(", ")),
        ]));
    }
    let id = Uuid::new_v4().to_string();
    let mut plan = plan.clone();
    plan.template_id = Some(id.clone());
    Ok(ImportTemplate {
        id,
        name: name.to_string(),
        plan,
        partner_id: None,
        origin: None,
        builtin: false,
        created_at: clock::today_iso(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::{ColumnBinding, DateOrder};
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;

    fn plan() -> ImportPlan {
        ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: [
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
            ]
            .into_iter()
            .map(|(index, header, field)| ColumnBinding {
                header: header.into(),
                index,
                field,
                decimal: None,
                date_order: None,
            })
            .collect(),
            template_id: None,
            date1904: false,
            pruefart: None,
        }
    }

    fn fresh(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let db = TrainsDb::load(&folder.config()).unwrap();
        (folder, db)
    }

    fn store(db: &mut TrainsDb, template: ImportTemplate) {
        db.transaction(|tx| {
            tx.put_template(template);
            Ok(())
        })
        .unwrap();
    }

    /// A shipped template is never changed; its user copy learns instead.
    #[test]
    fn a_confirmed_reading_on_a_builtin_goes_onto_a_user_copy() {
        let (_f, mut db) = fresh("learn-builtin");
        let mut confirmed = plan();
        confirmed.columns[0].header = "werk_ausg_ist".into();
        confirmed.columns[0].field = FieldKind::AuftragAusgangAm;
        confirmed.columns[0].date_order = Some(DateOrder::MonthFirst);

        let (copy, said) = learned(&db, "builtin:werkstattauftraege", &confirmed).unwrap();
        assert!(said.contains("Werkstattaufträge"));
        assert!(!copy.builtin);
        assert_eq!(copy.origin.as_deref(), Some("builtin:werkstattauftraege"));
        let datum = copy
            .plan
            .columns
            .iter()
            .find(|binding| binding.header == "werk_ausg_ist")
            .unwrap();
        assert_eq!(datum.date_order, Some(DateOrder::MonthFirst));
        assert_eq!(
            copy.plan.columns.len(),
            crate::trains::builtin::all()[0].plan.columns.len(),
            "the template's own columns are kept, not the file's"
        );
        store(&mut db, copy);

        // The copy is found again rather than a second one minted, and it
        // already knows everything this plan could teach it.
        assert!(learned(&db, "builtin:werkstattauftraege", &confirmed).is_none());
        assert_eq!(
            db.templates().iter().filter(|t| t.origin.is_some()).count(),
            1
        );
    }

    #[test]
    fn nothing_confirmed_means_nothing_learned() {
        let (_f, db) = fresh("learn-nothing");
        assert!(learned(&db, "builtin:telematik", &plan()).is_none());
    }

    #[test]
    fn a_user_template_learns_in_place() {
        let (_f, mut db) = fresh("learn-user");
        let saved = save("Monatsliste", &plan()).unwrap();
        let id = saved.id.clone();
        store(&mut db, saved);

        let mut confirmed = plan();
        confirmed.columns[1].date_order = Some(DateOrder::MonthFirst);
        let (template, _) = learned(&db, &id, &confirmed).unwrap();
        assert_eq!(template.id, id);
        store(&mut db, template);

        assert_eq!(
            db.template(&id).unwrap().plan.columns[1].date_order,
            Some(DateOrder::MonthFirst)
        );
        assert_eq!(db.templates().iter().filter(|t| !t.builtin).count(), 1);
    }

    #[test]
    fn saving_keeps_the_mapped_plan_and_points_it_at_itself() {
        let template = save("  Werkstatt Müller — Monatsliste ", &plan()).unwrap();
        assert_eq!(template.name, "Werkstatt Müller — Monatsliste");
        assert_eq!(template.plan.columns, plan().columns);
        assert_eq!(
            template.plan.template_id.as_deref(),
            Some(template.id.as_str())
        );
        assert!(!template.builtin);
    }

    #[test]
    fn a_template_needs_a_name_and_a_wagennummer() {
        assert!(save("  ", &plan()).is_err());
        let mut unmapped = plan();
        unmapped.columns[0].field = FieldKind::Ignorieren;
        let error = save("Monatsliste", &unmapped).unwrap_err();
        assert!(error.into_messages()[1].contains("Wagennummer"));
    }
}
