// ─── why ────────────────────────────────────────────────────────
// A row's names and numbers become references to entities — or an admission
// that they could not.
//
// NOTHING IS EVER CREATED IMPLICITLY. `New` and `Ambiguous` are not committable
// until the user picks or ticks, and `commit` re-checks every gate on its own
// side, because a stale preview must not be able to create four hundred
// partners. That is not caution about mistakes in general: a typo'd wagen
// number silently created becomes a permanent phantom that every later import
// matches against.
//
// A WAGEN may decide on its own; a RADSATZ may not. The twelve digits carry a
// check digit, so a hit is evidence. A Radsatznummer carries none and is not
// unique across senders, so its rule lives in `radsatz.rs` and is scoped by who
// sent the file.
//
// A wagen resolves on the canonical twelve digits. A miss falls back to the
// first ELEVEN — the check digit is the one place a typo is both likely and
// detectable, so a number that agrees on everything but that digit is a
// suggestion worth making, and exactly one match makes it `Likely`.
//
// That is also where the check-digit warning earns its keep. A wrong check digit
// alone is not evidence — real fleets carry them. A wrong check digit AND no
// known wagen is, and only the combination sends the row to the user.
// ────────────────────────────────────────────────────────────────

pub mod partner;
pub mod radsatz;

use std::collections::HashMap;

use crate::trains::db::TrainsDb;
use crate::trains::model::{MatchCandidate, PartnerRolle, Resolution};
use crate::trains::sanitise::format;

/// One staging run's answers, keyed on the RAW spelling as it stands in the
/// file. A monthly list names five or ten Werkstätten across two thousand rows,
/// and every miss otherwise costs a full scan of the partner table plus a
/// Levenshtein against each — three times per row, once per partner column.
#[derive(Default)]
pub struct PartnerMemo {
    seen: HashMap<(PartnerRolle, String), Resolution>,
}

impl PartnerMemo {
    pub fn find(&mut self, db: &TrainsDb, role: PartnerRolle, raw: Option<&str>) -> Resolution {
        let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
            return Resolution::Missing;
        };
        if let Some(hit) = self.seen.get(&(role, raw.to_string())) {
            return hit.clone();
        }
        let found = find_partner(db, role, Some(raw));
        self.seen.insert((role, raw.to_string()), found.clone());
        found
    }
}

pub fn wagen(db: &TrainsDb, uic: Option<&str>) -> Resolution {
    let Some(uic) = uic.filter(|value| !value.is_empty()) else {
        return Resolution::Missing;
    };

    if let Some(found) = db.wagen_by_nummer(uic) {
        return Resolution::Known {
            id: found.id.clone(),
            name: format::uic_display(&found.nummer),
        };
    }

    let stem = &uic[..uic.len().min(11)];
    let near: Vec<&crate::trains::model::Wagen> = db
        .wagen_starting_with(stem)
        .into_iter()
        .filter(|found| found.nummer != uic)
        .collect();

    match near.as_slice() {
        [only] => Resolution::Likely {
            id: only.id.clone(),
            name: format::uic_display(&only.nummer),
            hint: format!(
                "Prüfziffer weicht ab; vermutlich {}.",
                format::uic_display(&only.nummer)
            ),
        },
        [] => Resolution::New {
            proposal: format::uic_display(uic),
        },
        several => Resolution::Ambiguous {
            candidates: several
                .iter()
                .map(|found| MatchCandidate {
                    id: found.id.clone(),
                    name: format::uic_display(&found.nummer),
                    score: 90,
                    why: "Stimmt bis auf die Prüfziffer überein.".into(),
                })
                .collect(),
        },
    }
}

/// `sender` is who the file came from — the import template's partner, else the
/// row's confirmed Werkstatt. It is what makes a number decide or ask; see
/// `radsatz.rs` for why an unscoped number may not decide at all.
pub fn find_radsatz(db: &TrainsDb, raw: Option<&str>, sender: Option<&str>) -> Resolution {
    let Some(key) = raw
        .map(radsatz::match_key)
        .filter(|value| !value.is_empty())
    else {
        return Resolution::Missing;
    };

    let candidates = db.radsaetze_by_key(&key);

    if let Some(found) = candidates
        .iter()
        .find(|candidate| candidate.known_to(&key, sender))
    {
        return Resolution::Known {
            id: found.id.clone(),
            name: found.nummer.clone(),
        };
    }

    if !candidates.is_empty() {
        return Resolution::Ambiguous {
            candidates: candidates
                .iter()
                .map(|found| MatchCandidate {
                    id: found.id.clone(),
                    name: found.nummer.clone(),
                    score: 70,
                    why: "Gleiche Nummer, aber von einem anderen Absender bestätigt.".into(),
                })
                .collect(),
        };
    }

    let stripped = radsatz::without_leading_zeros(&key);
    let zero_variants = db.radsaetze_without_leading_zeros(stripped);
    match zero_variants.as_slice() {
        [only] => Resolution::Likely {
            id: only.id.clone(),
            name: only.nummer.clone(),
            hint: format!(
                "Unterscheidet sich nur in führenden Nullen von „{}“.",
                only.nummer
            ),
        },
        [] => Resolution::New { proposal: key },
        several => Resolution::Ambiguous {
            candidates: several
                .iter()
                .map(|found| MatchCandidate {
                    id: found.id.clone(),
                    name: found.nummer.clone(),
                    score: 60,
                    why: "Unterscheidet sich nur in führenden Nullen.".into(),
                })
                .collect(),
        },
    }
}

pub fn find_partner(db: &TrainsDb, role: PartnerRolle, raw: Option<&str>) -> Resolution {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Resolution::Missing;
    };
    let key = partner::match_key(raw);
    if key.is_empty() {
        return Resolution::Missing;
    }

    if let Some(found) = db.partner_by_key(&key) {
        return Resolution::Known {
            id: found.id.clone(),
            name: found.name.clone(),
        };
    }

    let mut scored: Vec<(f64, &crate::trains::model::Partner)> = db
        .partner_with_rolle(role)
        .into_iter()
        .map(|candidate| (partner::similarity(&key, &candidate.match_key), candidate))
        .filter(|(score, _)| partner::is_likely(*score))
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));

    match scored.as_slice() {
        [] => Resolution::New {
            proposal: raw.to_string(),
        },
        [(_, only)] => Resolution::Likely {
            id: only.id.clone(),
            name: only.name.clone(),
            hint: format!("Ähnlich geschrieben wie „{}“.", only.name),
        },
        several => Resolution::Ambiguous {
            candidates: several
                .iter()
                .map(|(score, candidate)| MatchCandidate {
                    id: candidate.id.clone(),
                    name: candidate.name.clone(),
                    score: (score * 100.0).round() as u8,
                    why: format!("Ähnlich geschrieben wie „{raw}“."),
                })
                .collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::{Partner, Radsatz, Wagen};

    fn wagen_row(id: &str, uic: &str) -> Wagen {
        Wagen {
            id: id.into(),
            nummer: uic.into(),
            halter_id: None,
            eigentuemer_id: None,
            bauart: None,
            bemerkung: None,
            created_at: "2026-08-16".into(),
            source: None,
        }
    }

    fn partner_row(id: &str, name: &str) -> Partner {
        Partner {
            id: id.into(),
            rollen: vec![PartnerRolle::Werkstatt],
            name: name.into(),
            match_key: partner::match_key(name),
            aliases: Vec::new(),
            created_at: "2026-08-16".into(),
            bemerkung: None,
        }
    }

    fn radsatz_row(id: &str, nummer: &str) -> Radsatz {
        Radsatz {
            id: id.into(),
            nummer: nummer.into(),
            match_key: radsatz::match_key(nummer),
            aliases: Vec::new(),
            wellennummer: None,
            system_id: None,
            bauart: None,
            bemerkung: None,
            created_at: "2026-08-16".into(),
            source: None,
        }
    }

    /// Seeds a radsatz already confirmed for `sender`, which is the state every
    /// committed import leaves behind.
    fn with_radsatz(
        label: &str,
        id: &str,
        nummer: &str,
        sender: Option<&str>,
    ) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_radsatz(radsatz_row(id, nummer));
            tx.learn_radsatz_alias(id, &radsatz::match_key(nummer), sender);
            Ok(())
        })
        .unwrap();
        (folder, db)
    }

    fn seeded(label: &str, wagen: &[Wagen], partners: &[Partner]) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            for wagen in wagen {
                tx.put_wagen(wagen.clone());
            }
            for partner in partners {
                tx.put_partner(partner.clone());
            }
            Ok(())
        })
        .unwrap();
        (folder, db)
    }

    /// The memo is an optimisation and may not be a behaviour: the same raw
    /// spelling has to come back with exactly what the direct call answers.
    #[test]
    fn the_memo_answers_what_the_direct_call_would() {
        let (_f, db) = seeded(
            "res-memo",
            &[],
            &[partner_row("p1", "Müller GmbH"), partner_row("p2", "Bahn")],
        );
        let mut memo = PartnerMemo::default();
        for raw in ["Müller GmbH", "Mueller  gmbh", "Unbekannt", "", "Bahn"] {
            let direct = find_partner(&db, PartnerRolle::Werkstatt, Some(raw));
            assert_eq!(memo.find(&db, PartnerRolle::Werkstatt, Some(raw)), direct);
            assert_eq!(
                memo.find(&db, PartnerRolle::Werkstatt, Some(raw)),
                direct,
                "the second ask is the cached one"
            );
        }
    }

    /// The role is part of the key: the same name asked for as a Werkstatt and
    /// as a Halter are two different questions.
    #[test]
    fn the_memo_does_not_answer_across_roles() {
        let (_f, db) = seeded("res-memo-role", &[], &[partner_row("p1", "Müller GmbH")]);
        let mut memo = PartnerMemo::default();
        assert_eq!(
            memo.find(&db, PartnerRolle::Halter, Some("Müller GmbH")),
            find_partner(&db, PartnerRolle::Halter, Some("Müller GmbH"))
        );
    }

    #[test]
    fn a_known_wagen_resolves_on_its_canonical_digits() {
        let (_f, db) = seeded("res-known", &[wagen_row("w1", "318047401234")], &[]);
        assert_eq!(
            wagen(&db, Some("318047401234")),
            Resolution::Known {
                id: "w1".into(),
                name: "31 80 4740 123-4".into()
            }
        );
    }

    /// The check digit is where a typo is both likely and detectable, so a
    /// number agreeing on everything else is worth suggesting.
    #[test]
    fn a_number_differing_only_in_its_check_digit_is_likely() {
        let (_f, db) = seeded("res-near", &[wagen_row("w1", "318047401233")], &[]);
        let resolution = wagen(&db, Some("318047401234"));
        let Resolution::Likely { id, hint, .. } = resolution else {
            panic!("expected Likely, got {resolution:?}");
        };
        assert_eq!(id, "w1");
        assert!(hint.contains("Prüfziffer"), "{hint}");
    }

    #[test]
    fn an_unknown_wagen_is_proposed_rather_than_created() {
        let (_f, db) = seeded("res-new", &[], &[]);
        assert_eq!(
            wagen(&db, Some("318047401234")),
            Resolution::New {
                proposal: "31 80 4740 123-4".into()
            }
        );
    }

    #[test]
    fn an_empty_cell_is_missing_not_new() {
        let (_f, db) = seeded("res-missing", &[], &[]);
        assert_eq!(wagen(&db, None), Resolution::Missing);
        assert_eq!(wagen(&db, Some("")), Resolution::Missing);
        assert_eq!(
            find_partner(&db, PartnerRolle::Werkstatt, Some("  ")),
            Resolution::Missing
        );
    }

    /// Tier one: an exact key hit decides, with no scoring involved.
    #[test]
    fn a_partner_resolves_through_its_normalised_name() {
        let (_f, db) = seeded("res-partner", &[], &[partner_row("p1", "Müller GmbH")]);
        assert_eq!(
            find_partner(
                &db,
                PartnerRolle::Werkstatt,
                Some("Fa. Mueller GmbH & Co. KG")
            ),
            Resolution::Known {
                id: "p1".into(),
                name: "Müller GmbH".into()
            }
        );
    }

    /// A learnt alias is what makes the second file from a sender free.
    #[test]
    fn a_learnt_alias_resolves_at_tier_one() {
        let folder = TempDir::new("res-alias");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_partner(partner_row("p1", "Müller GmbH"));
            tx.learn_alias("p1", &partner::match_key("Werkstatt Nord"));
            Ok(())
        })
        .unwrap();
        assert_eq!(
            find_partner(&db, PartnerRolle::Werkstatt, Some("Werkstatt Nord")),
            Resolution::Known {
                id: "p1".into(),
                name: "Müller GmbH".into()
            }
        );
    }

    #[test]
    fn a_near_miss_partner_is_suggested_never_matched_silently() {
        let (_f, db) = seeded("res-likely", &[], &[partner_row("p1", "Bahnwerk")]);
        let resolution = find_partner(&db, PartnerRolle::Werkstatt, Some("Bahnwerkk"));
        assert!(
            matches!(resolution, Resolution::Likely { ref id, .. } if id == "p1"),
            "{resolution:?}"
        );
    }

    #[test]
    fn a_partner_in_the_other_role_is_not_offered() {
        let (_f, db) = seeded("res-role", &[], &[partner_row("p1", "Bahnwerk")]);
        assert_eq!(
            find_partner(&db, PartnerRolle::Halter, Some("Bahnwerkk")),
            Resolution::New {
                proposal: "Bahnwerkk".into()
            }
        );
    }

    #[test]
    fn nothing_resolvable_needs_the_user_before_it_can_be_committed() {
        let (_f, db) = seeded("res-gate", &[], &[]);
        assert!(wagen(&db, Some("318047401234")).needs_input());
        assert!(find_partner(&db, PartnerRolle::Werkstatt, Some("Neu")).needs_input());
        assert!(!wagen(&db, None).needs_input());
    }

    /// Tier one: the sender who introduced the number is recognised without a
    /// question. This is the loop that makes the monthly file free.
    #[test]
    fn the_sender_who_confirmed_a_number_is_answered_without_a_question() {
        let (_f, db) = with_radsatz("rs-known", "r1", "RS4711", Some("p1"));
        assert_eq!(
            find_radsatz(&db, Some("rs 4711"), Some("p1")),
            Resolution::Known {
                id: "r1".into(),
                name: "RS4711".into()
            }
        );
    }

    /// THE INVARIANT THIS WHOLE STEP EXISTS FOR. Two workshops legitimately use
    /// one number for two different radsaetze, so an identical number from a
    /// sender who never confirmed it is a QUESTION, never a silent merge.
    #[test]
    fn the_same_number_from_another_sender_is_a_question_not_a_match() {
        let (_f, db) = with_radsatz("rs-other", "r1", "RS4711", Some("p1"));
        let resolution = find_radsatz(&db, Some("RS4711"), Some("p2"));
        let Resolution::Ambiguous { candidates } = resolution else {
            panic!("expected Ambiguous, got {resolution:?}");
        };
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "r1");
        assert!(find_radsatz(&db, Some("RS4711"), Some("p2")).needs_input());
    }

    /// An unknown sender can never be given a `Known` off a bare number — that
    /// is the safe direction, and it is why the scope is `Option`.
    #[test]
    fn a_row_with_no_known_sender_is_never_answered_off_a_bare_number() {
        let (_f, db) = with_radsatz("rs-nosender", "r1", "RS4711", Some("p1"));
        assert!(find_radsatz(&db, Some("RS4711"), None).needs_input());
    }

    /// Confirming teaches the second sender, and MUST NOT disturb the first.
    #[test]
    fn confirming_teaches_one_sender_and_leaves_the_other_alone() {
        let (folder, mut db) = with_radsatz("rs-learn", "r1", "RS4711", Some("p1"));
        db.transaction(|tx| {
            tx.learn_radsatz_alias("r1", "RS4711", Some("p2"));
            Ok(())
        })
        .unwrap();
        let _ = &folder;

        for sender in ["p1", "p2"] {
            assert_eq!(
                find_radsatz(&db, Some("RS4711"), Some(sender)),
                Resolution::Known {
                    id: "r1".into(),
                    name: "RS4711".into()
                },
                "{sender}"
            );
        }
    }

    /// Choosing "neu" instead leaves TWO radsaetze under one key, and neither
    /// steals the other's sender. One key therefore has to hold several ids.
    #[test]
    fn two_radsaetze_may_share_a_number_under_different_senders() {
        let folder = TempDir::new("rs-collide");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_radsatz(radsatz_row("r1", "RS4711"));
            tx.learn_radsatz_alias("r1", "RS4711", Some("p1"));
            tx.put_radsatz(radsatz_row("r2", "RS4711"));
            tx.learn_radsatz_alias("r2", "RS4711", Some("p2"));
            Ok(())
        })
        .unwrap();

        assert_eq!(db.radsaetze_by_key("RS4711").len(), 2);
        assert!(
            matches!(find_radsatz(&db, Some("RS4711"), Some("p1")), Resolution::Known { ref id, .. } if id == "r1")
        );
        assert!(
            matches!(find_radsatz(&db, Some("RS4711"), Some("p2")), Resolution::Known { ref id, .. } if id == "r2")
        );
    }

    /// Leading zeros are a formatting difference, not a digit — Excel eats them
    /// on its own. So it is a suggestion, and never decides.
    #[test]
    fn a_leading_zero_variant_is_suggested_and_never_matched() {
        let (_f, db) = with_radsatz("rs-zeros", "r1", "0012345", Some("p1"));
        let resolution = find_radsatz(&db, Some("12345"), Some("p1"));
        let Resolution::Likely { id, hint, .. } = resolution else {
            panic!("expected Likely, got {resolution:?}");
        };
        assert_eq!(id, "r1");
        assert!(hint.contains("Nullen"), "{hint}");
    }

    /// A differing DIGIT is a different radsatz, and nothing may suggest
    /// otherwise — that would put the wrong history under a wagen.
    #[test]
    fn a_number_differing_by_a_digit_is_never_suggested() {
        let (_f, db) = with_radsatz("rs-digit", "r1", "RS4711", Some("p1"));
        assert_eq!(
            find_radsatz(&db, Some("RS4712"), Some("p1")),
            Resolution::New {
                proposal: "RS4712".into()
            }
        );
    }

    #[test]
    fn an_empty_radsatz_cell_is_missing_not_new() {
        let (_f, db) = seeded("rs-missing", &[], &[]);
        assert_eq!(find_radsatz(&db, None, Some("p1")), Resolution::Missing);
        assert_eq!(
            find_radsatz(&db, Some(" -- "), Some("p1")),
            Resolution::Missing
        );
    }
}
