use outflow_core::vocabulary::{
    MAX_VOCABULARY_ENTRIES, VocabularyCollectionError, VocabularyEntryRef,
    validate_vocabulary_collection, validate_vocabulary_entry,
};
use outflow_core::vocabulary_matching::{CURRENT_VOCABULARY_MATCHING_VERSION, comparison_key};
use std::collections::HashMap;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

const MEASURED_RUNS: usize = 7;

fn main() -> ExitCode {
    if cfg!(debug_assertions) {
        eprintln!("run this benchmark with --release");
        return ExitCode::FAILURE;
    }

    println!("Synthetic vocabulary matching benchmark");
    println!(
        "policy={}; release profile; measured_runs={MEASURED_RUNS}; warmup=2,000 key calls / 100 validation calls",
        CURRENT_VOCABULARY_MATCHING_VERSION.label()
    );
    println!("median nanoseconds per operation; ratio is ICU4X / lowercase");

    for (script, phrase) in [
        ("ascii", "North Parkade Rust phrase 123"),
        ("multilingual", "Straße CAFÉ Σς İı Москва 東京 हिन्दी 😀 𐐀"),
    ] {
        let baseline_phrase = phrase;
        let icu_phrase = phrase;
        benchmark(
            &format!("{script}/individual_key"),
            20_000,
            2_000,
            || {
                black_box(black_box(baseline_phrase).to_lowercase());
            },
            || {
                black_box(comparison_key(
                    black_box(icu_phrase),
                    CURRENT_VOCABULARY_MATCHING_VERSION,
                ));
            },
        );
    }

    for script in ["ascii", "multilingual"] {
        for count in [1, 100, MAX_VOCABULARY_ENTRIES] {
            let owned = make_entries(script, count, false);
            let entries = entry_refs(&owned);
            let iterations = match count {
                1 => 10_000,
                100 => 1_000,
                _ => 200,
            };
            benchmark(
                &format!("{script}/valid_collection/{count}"),
                iterations,
                100,
                || {
                    let _ = black_box(legacy_validate_collection(black_box(&entries)));
                },
                || {
                    let _ = black_box(validate_vocabulary_collection(black_box(&entries)));
                },
            );
        }

        for count in [100, MAX_VOCABULARY_ENTRIES] {
            let owned = make_entries(script, count, true);
            let entries = entry_refs(&owned);
            let iterations = if count == 100 { 1_000 } else { 200 };
            benchmark(
                &format!("{script}/duplicate_at_last_entry/{count}"),
                iterations,
                100,
                || {
                    let _ = black_box(legacy_validate_collection(black_box(&entries)));
                },
                || {
                    let _ = black_box(validate_vocabulary_collection(black_box(&entries)));
                },
            );
        }
    }

    ExitCode::SUCCESS
}

fn make_entries(script: &str, count: usize, duplicate_first_at_end: bool) -> Vec<(String, String)> {
    let unique_count = if duplicate_first_at_end {
        count - 1
    } else {
        count
    };
    let mut entries = (0..unique_count)
        .map(|index| {
            let spoken = if script == "ascii" {
                format!("dictation term {index}")
            } else {
                format!("Straße café Σσ Москва 東京 हिन्दी phrase {index}")
            };
            (spoken, format!("replacement {index}"))
        })
        .collect::<Vec<_>>();

    if duplicate_first_at_end {
        let spoken = entries[0].0.clone();
        entries.push((spoken, String::from("different final replacement")));
    }

    entries
}

fn entry_refs(entries: &[(String, String)]) -> Vec<VocabularyEntryRef<'_>> {
    entries
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect()
}

// Mirrors the pre-Task 45 validator's ordering and lowercase key calculation.
fn legacy_validate_collection(
    entries: &[VocabularyEntryRef<'_>],
) -> Result<(), VocabularyCollectionError> {
    if entries.len() > MAX_VOCABULARY_ENTRIES {
        return Err(VocabularyCollectionError::TooManyEntries {
            count: entries.len(),
            maximum: MAX_VOCABULARY_ENTRIES,
        });
    }

    for (index, entry) in entries.iter().enumerate() {
        validate_vocabulary_entry(entry.spoken, entry.written)
            .map_err(|source| VocabularyCollectionError::InvalidEntry { index, source })?;
    }

    let mut first_index_by_key = HashMap::<String, usize>::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let key = entry.spoken.to_lowercase();
        if let Some(&first_index) = first_index_by_key.get(&key) {
            return Err(VocabularyCollectionError::DuplicateSpoken {
                first_index,
                duplicate_index: index,
            });
        }
        first_index_by_key.insert(key, index);
    }

    Ok(())
}

fn benchmark<B, I>(label: &str, iterations: usize, warmup: usize, mut baseline: B, mut icu: I)
where
    B: FnMut(),
    I: FnMut(),
{
    for _ in 0..warmup {
        baseline();
        icu();
    }

    let mut baseline_samples = Vec::with_capacity(MEASURED_RUNS);
    let mut icu_samples = Vec::with_capacity(MEASURED_RUNS);
    for run in 0..MEASURED_RUNS {
        if run % 2 == 0 {
            baseline_samples.push(measure(&mut baseline, iterations));
            icu_samples.push(measure(&mut icu, iterations));
        } else {
            icu_samples.push(measure(&mut icu, iterations));
            baseline_samples.push(measure(&mut baseline, iterations));
        }
    }

    let baseline_median = median(&mut baseline_samples);
    let icu_median = median(&mut icu_samples);
    println!(
        "case={label}; iterations_per_run={iterations}; lowercase_ns_per_op={baseline_median:.2}; icu_ns_per_op={icu_median:.2}; ratio={:.2}x",
        icu_median / baseline_median
    );
}

fn measure<F: FnMut()>(run: &mut F, iterations: usize) -> f64 {
    let started = Instant::now();
    for _ in 0..iterations {
        run();
    }
    started.elapsed().as_nanos() as f64 / iterations as f64
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}
