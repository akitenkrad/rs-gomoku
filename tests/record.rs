use rs_gomoku::game::{Outcome, Rule, Side, Stone, parse_cell};
use rs_gomoku::record::{Record, RecordError, parse, to_text};

fn assert_error(text: &str, expected: RecordError, display: &str) {
    let error = parse(text).unwrap_err();
    assert_eq!(error, expected);
    assert_eq!(error.to_string(), display);
}

#[test]
fn parses_headers_case_insensitive_moves_comments_and_defaults() {
    let record = parse(
        "# rs-gomoku record v1\n\
         size = 9 # small board\n\
         rule = standard\n\
         first = cpu\n\
         human = Alice\n\
         cpu = Bob\n\n\
         A1\n\
         i9\n",
    )
    .unwrap();
    assert_eq!(record.size, 9);
    assert_eq!(record.rule, Rule::Standard);
    assert_eq!(record.first, Side::Cpu);
    assert_eq!(record.names, [Some("Alice".into()), Some("Bob".into())]);
    assert_eq!(record.moves, [0, 80]);

    let defaults = parse("h8\ni9\n").unwrap();
    assert_eq!(defaults.size, 15);
    assert_eq!(defaults.rule, Rule::Freestyle);
    assert_eq!(defaults.first, Side::Human);
    assert_eq!(defaults.names, [None, None]);
}

#[test]
fn rejects_unknown_duplicate_late_and_invalid_headers_with_lines() {
    assert_error(
        "mystery = x\n",
        RecordError::UnknownKey {
            line: 1,
            key: "mystery".into(),
        },
        "line 1: unknown key 'mystery'",
    );
    assert_error(
        "size = 15\nsize = 9\n",
        RecordError::DuplicateKey { line: 2, key: "size".into() },
        "line 2: duplicate key 'size'",
    );
    assert_error(
        "a1\nrule = standard\n",
        RecordError::HeaderAfterMove { line: 2 },
        "line 2: header after first move",
    );
    assert_error(
        "size = 6\n",
        RecordError::InvalidSize { line: 1, value: "6".into() },
        "line 1: invalid board size '6'",
    );
    assert_error(
        "size = twenty\n",
        RecordError::InvalidSize {
            line: 1,
            value: "twenty".into(),
        },
        "line 1: invalid board size 'twenty'",
    );
    assert_error(
        "rule = renju\n",
        RecordError::InvalidRule {
            line: 1,
            value: "renju".into(),
        },
        "line 1: invalid rule 'renju'",
    );
    assert_error(
        "first = robot\n",
        RecordError::InvalidFirst {
            line: 1,
            value: "robot".into(),
        },
        "line 1: invalid first side 'robot'",
    );
    assert_error(
        "human = # absent\n",
        RecordError::EmptyName {
            line: 1,
            key: "human".into(),
        },
        "line 1: human name must not be empty",
    );
}

#[test]
fn rejects_invalid_outside_occupied_and_post_game_moves() {
    for invalid in ["a0", "a01", "j9", "aa1", "1a", "１"] {
        assert_error(
            &format!("size = 9\n{invalid}\n"),
            RecordError::InvalidMove {
                line: 2,
                value: invalid.into(),
            },
            &format!("line 2: invalid move '{invalid}'"),
        );
    }
    assert_error(
        "a1\nA1\n",
        RecordError::CellOccupied { line: 2, cell: 0, size: 15 },
        "line 2: cell a1 is already occupied",
    );
    assert_error(
        "h8\nH8\n",
        RecordError::CellOccupied {
            line: 2,
            cell: 112,
            size: 15,
        },
        "line 2: cell h8 is already occupied",
    );
    assert_error(
        "a1\na2\nb1\nb2\nc1\nc2\nd1\nd2\ne1\nf1\n",
        RecordError::MoveAfterGameOver { line: 10 },
        "line 10: move after game over",
    );
}

#[test]
fn canonical_text_round_trips_and_game_at_reconstructs_every_ply() {
    let original = Record {
        size: 11,
        rule: Rule::Standard,
        first: Side::Cpu,
        names: [Some("Alice Smith".into()), Some("Bob CPU".into())],
        moves: vec![0, 60, 120],
    };
    let text = to_text(&original);
    assert!(text.starts_with("# rs-gomoku record v1\n"));
    assert!(text.contains("size  = 11\n"));
    assert!(text.contains("rule  = standard\n"));
    assert!(text.contains("first = cpu\n"));
    assert!(text.ends_with("a1\nf6\nk11\n"));
    assert_eq!(parse(&text).unwrap(), original);

    for ply in 0..=original.moves.len() {
        let game = original.game_at(ply).unwrap();
        assert_eq!(game.history.len(), ply);
        assert_eq!(game.size, 11);
        assert_eq!(game.rule, Rule::Standard);
        assert_eq!(game.to_move, if ply % 2 == 0 { Side::Cpu } else { Side::Human });
        if ply > 0 {
            assert_eq!(game.board[original.moves[0]], Some(Stone::Black));
        }
    }
    assert!(original.game_at(4).is_none());
}

#[test]
fn parses_all_example_records_and_preserves_standard_overline() {
    let examples = [
        include_str!("../examples/records/black-wins-15.gmkrec"),
        include_str!("../examples/records/small-9.gmkrec"),
        include_str!("../examples/records/overline-standard.gmkrec"),
        include_str!("../examples/records/unfinished.gmkrec"),
    ];
    let records = examples.map(|text| parse(text).unwrap());
    assert!(matches!(
        records[0].game_at(records[0].moves.len()).unwrap().outcome(),
        Some(Outcome::Win { side: Side::Human, .. })
    ));
    let four_three = records[0].game_at(11).unwrap();
    assert_eq!(four_three.board[7 * 15 + 5..=7 * 15 + 8], [Some(Stone::Black); 4]);
    for row in 5..=7 {
        assert_eq!(four_three.board[row * 15 + 8], Some(Stone::Black));
    }
    assert_eq!(four_three.board[7 * 15 + 4], Some(Stone::White));
    assert_eq!(four_three.board[7 * 15 + 9], None);
    assert_eq!(four_three.board[4 * 15 + 8], None);
    assert_eq!(four_three.board[8 * 15 + 8], None);
    assert_eq!(records[1].size, 9);
    let overline = &records[2];
    assert_eq!(overline.rule, Rule::Standard);
    assert_eq!(overline.game_at(11).unwrap().outcome(), None);
    assert!(overline.moves.len() > 11, "the record must continue after the overline");
    assert_eq!(records[3].game_at(records[3].moves.len()).unwrap().outcome(), None);
}

#[test]
fn coordinate_parser_obeys_record_size() {
    assert_eq!(parse_cell("i9", 9), Some(80));
    assert_eq!(parse_cell("j9", 9), None);
}
