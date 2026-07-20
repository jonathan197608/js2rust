// src/main.rs -- MDN test project CLI
// Usage:
//   mdn-test-project                    Run all fragments with Node.js comparison (default)
//   mdn-test-project --list             List all available fragments
//   mdn-test-project <fragment_name>    Run a single fragment
//   mdn-test-project --all              Same as default (run all fragments)
use js2rust_bridge::js2rust_bridge;
use std::env;
use std::process::Command;

js2rust_bridge!();

// Flush C stdio buffers (Zig runtime writes via C FFI, not Rust stdout).
// Without this, stdout is fully buffered when piped and output is lost on exit.
extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

fn flush_stdout() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

/// Path to js_src directory (resolved at compile time via CARGO_MANIFEST_DIR).
const JS_SRC_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/js_src");

/// All fragment names, in pass_fragments.json order (statements, expressions, builtins).
const ALL_FRAGMENTS: &[&str] = &[
    // --- statements (8) ---
    "test_statements_frag_0",
    "test_statements_frag_3",
    "test_statements_frag_11",
    "test_statements_frag_12",
    "test_statements_frag_15",
    "test_statements_frag_18",
    "test_statements_frag_27",
    "test_statements_frag_37",
    // --- expressions (134) ---
    "test_expressions_frag_1",
    "test_expressions_frag_2",
    "test_expressions_frag_3",
    "test_expressions_frag_4",
    "test_expressions_frag_7",
    "test_expressions_frag_8",
    "test_expressions_frag_9",
    "test_expressions_frag_10",
    "test_expressions_frag_12",
    "test_expressions_frag_13",
    "test_expressions_frag_14",
    "test_expressions_frag_15",
    "test_expressions_frag_17",
    "test_expressions_frag_18",
    "test_expressions_frag_19",
    "test_expressions_frag_20",
    "test_expressions_frag_21",
    "test_expressions_frag_22",
    "test_expressions_frag_24",
    "test_expressions_frag_25",
    "test_expressions_frag_26",
    "test_expressions_frag_27",
    "test_expressions_frag_28",
    "test_expressions_frag_29",
    "test_expressions_frag_30",
    "test_expressions_frag_31",
    "test_expressions_frag_32",
    "test_expressions_frag_33",
    "test_expressions_frag_34",
    "test_expressions_frag_35",
    "test_expressions_frag_36",
    "test_expressions_frag_37",
    "test_expressions_frag_38",
    "test_expressions_frag_39",
    "test_expressions_frag_40",
    "test_expressions_frag_41",
    "test_expressions_frag_42",
    "test_expressions_frag_43",
    "test_expressions_frag_44",
    "test_expressions_frag_45",
    "test_expressions_frag_46",
    "test_expressions_frag_47",
    "test_expressions_frag_48",
    "test_expressions_frag_49",
    "test_expressions_frag_50",
    "test_expressions_frag_51",
    "test_expressions_frag_52",
    "test_expressions_frag_53",
    "test_expressions_frag_54",
    "test_expressions_frag_55",
    "test_expressions_frag_56",
    "test_expressions_frag_57",
    "test_expressions_frag_58",
    "test_expressions_frag_59",
    "test_expressions_frag_60",
    "test_expressions_frag_61",
    "test_expressions_frag_62",
    "test_expressions_frag_63",
    "test_expressions_frag_64",
    "test_expressions_frag_65",
    "test_expressions_frag_66",
    "test_expressions_frag_67",
    "test_expressions_frag_69",
    "test_expressions_frag_71",
    "test_expressions_frag_72",
    "test_expressions_frag_73",
    "test_expressions_frag_76",
    "test_expressions_frag_77",
    "test_expressions_frag_80",
    "test_expressions_frag_82",
    "test_expressions_frag_83",
    "test_expressions_frag_84",
    "test_expressions_frag_85",
    "test_expressions_frag_86",
    "test_expressions_frag_87",
    "test_expressions_frag_88",
    "test_expressions_frag_89",
    "test_expressions_frag_90",
    "test_expressions_frag_92",
    "test_expressions_frag_93",
    "test_expressions_frag_94",
    "test_expressions_frag_95",
    "test_expressions_frag_96",
    "test_expressions_frag_97",
    "test_expressions_frag_99",
    "test_expressions_frag_100",
    "test_expressions_frag_102",
    "test_expressions_frag_103",
    "test_expressions_frag_104",
    "test_expressions_frag_106",
    "test_expressions_frag_109",
    "test_expressions_frag_110",
    "test_expressions_frag_111",
    "test_expressions_frag_112",
    "test_expressions_frag_113",
    "test_expressions_frag_114",
    "test_expressions_frag_115",
    "test_expressions_frag_117",
    "test_expressions_frag_119",
    "test_expressions_frag_121",
    "test_expressions_frag_122",
    "test_expressions_frag_123",
    "test_expressions_frag_124",
    "test_expressions_frag_125",
    "test_expressions_frag_126",
    "test_expressions_frag_128",
    "test_expressions_frag_129",
    "test_expressions_frag_131",
    "test_expressions_frag_132",
    "test_expressions_frag_133",
    "test_expressions_frag_134",
    "test_expressions_frag_135",
    "test_expressions_frag_136",
    "test_expressions_frag_137",
    "test_expressions_frag_138",
    "test_expressions_frag_139",
    "test_expressions_frag_141",
    "test_expressions_frag_142",
    "test_expressions_frag_143",
    "test_expressions_frag_144",
    "test_expressions_frag_145",
    "test_expressions_frag_146",
    "test_expressions_frag_148",
    "test_expressions_frag_149",
    "test_expressions_frag_150",
    "test_expressions_frag_151",
    "test_expressions_frag_152",
    "test_expressions_frag_153",
    "test_expressions_frag_155",
    "test_expressions_frag_156",
    "test_expressions_frag_157",
    "test_expressions_frag_158",
    "test_expressions_frag_159",
    "test_expressions_frag_160",
    // --- builtins (97) ---
    "test_builtins_frag_0",
    "test_builtins_frag_3",
    "test_builtins_frag_8",
    "test_builtins_frag_16",
    "test_builtins_frag_17",
    "test_builtins_frag_18",
    "test_builtins_frag_32",
    "test_builtins_frag_34",
    "test_builtins_frag_35",
    "test_builtins_frag_37",
    "test_builtins_frag_39",
    "test_builtins_frag_40",
    "test_builtins_frag_41",
    "test_builtins_frag_43",
    "test_builtins_frag_44",
    "test_builtins_frag_46",
    "test_builtins_frag_48",
    "test_builtins_frag_49",
    "test_builtins_frag_50",
    "test_builtins_frag_51",
    "test_builtins_frag_52",
    "test_builtins_frag_53",
    "test_builtins_frag_54",
    "test_builtins_frag_55",
    "test_builtins_frag_56",
    "test_builtins_frag_57",
    "test_builtins_frag_60",
    "test_builtins_frag_61",
    "test_builtins_frag_68",
    "test_builtins_frag_71",
    "test_builtins_frag_74",
    "test_builtins_frag_80",
    "test_builtins_frag_81",
    "test_builtins_frag_95",
    "test_builtins_frag_96",
    "test_builtins_frag_97",
    "test_builtins_frag_98",
    "test_builtins_frag_100",
    "test_builtins_frag_101",
    "test_builtins_frag_102",
    "test_builtins_frag_103",
    "test_builtins_frag_104",
    "test_builtins_frag_105",
    "test_builtins_frag_106",
    "test_builtins_frag_107",
    "test_builtins_frag_110",
    "test_builtins_frag_113",
    "test_builtins_frag_115",
    "test_builtins_frag_116",
    "test_builtins_frag_126",
    "test_builtins_frag_130",
    "test_builtins_frag_137",
    "test_builtins_frag_150",
    "test_builtins_frag_151",
    "test_builtins_frag_152",
    "test_builtins_frag_153",
    "test_builtins_frag_154",
    "test_builtins_frag_156",
    "test_builtins_frag_157",
    "test_builtins_frag_158",
    "test_builtins_frag_159",
    "test_builtins_frag_160",
    "test_builtins_frag_161",
    "test_builtins_frag_162",
    "test_builtins_frag_163",
    "test_builtins_frag_164",
    "test_builtins_frag_165",
    "test_builtins_frag_166",
    "test_builtins_frag_167",
    "test_builtins_frag_168",
    "test_builtins_frag_169",
    "test_builtins_frag_170",
    "test_builtins_frag_172",
    "test_builtins_frag_175",
    "test_builtins_frag_176",
    "test_builtins_frag_177",
    "test_builtins_frag_179",
    "test_builtins_frag_182",
    "test_builtins_frag_202",
    "test_builtins_frag_204",
    "test_builtins_frag_205",
    "test_builtins_frag_209",
    "test_builtins_frag_210",
    "test_builtins_frag_211",
    "test_builtins_frag_212",
    "test_builtins_frag_213",
    "test_builtins_frag_214",
    "test_builtins_frag_215",
    "test_builtins_frag_216",
    "test_builtins_frag_218",
    "test_builtins_frag_219",
    "test_builtins_frag_223",
    "test_builtins_frag_224",
    "test_builtins_frag_225",
    "test_builtins_frag_226",
    "test_builtins_frag_227",
    "test_builtins_frag_228",
];

fn main() {
    let args: Vec<String> = env::args().collect();
    let binary = args[0].clone();

    if args.len() < 2 {
        // Default: run all fragments with Node.js comparison
        run_all(&binary);
        return;
    }

    match args[1].as_str() {
        "--list" => {
            for frag in ALL_FRAGMENTS {
                println!("{}", frag);
            }
        }
        "--all" => {
            run_all(&binary);
        }
        frag => {
            js2rust_init();
            if !run_fragment(frag) {
                eprintln!("Unknown fragment: {}", frag);
                eprintln!("Use --list to see available fragments.");
                flush_stdout();
                js2rust_deinit();
                std::process::exit(1);
            }
            flush_stdout();
            js2rust_deinit();
        }
    }
}

/// Dispatch to a single bridge function. Returns false if fragment name is unknown.
/// `let _ =` is intentionally used for all bridge calls to uniformly discard
/// both `()` and `Result` return types.
#[allow(clippy::let_unit_value)]
fn run_fragment(frag: &str) -> bool {
    match frag {
        "test_statements_frag_0" => {
            let _ = testStatements_frag_0();
            true
        }
        "test_statements_frag_3" => {
            let _ = testStatements_frag_3();
            true
        }
        "test_statements_frag_11" => {
            let _ = testStatements_frag_11();
            true
        }
        "test_statements_frag_12" => {
            let _ = testStatements_frag_12();
            true
        }
        "test_statements_frag_15" => {
            let _ = testStatements_frag_15();
            true
        }
        "test_statements_frag_18" => {
            let _ = testStatements_frag_18();
            true
        }
        "test_statements_frag_27" => {
            let _ = testStatements_frag_27();
            true
        }
        "test_statements_frag_37" => {
            let _ = testStatements_frag_37();
            true
        }
        "test_expressions_frag_1" => {
            let _ = testExpressions_frag_1();
            true
        }
        "test_expressions_frag_2" => {
            let _ = testExpressions_frag_2();
            true
        }
        "test_expressions_frag_3" => {
            let _ = testExpressions_frag_3();
            true
        }
        "test_expressions_frag_4" => {
            let _ = testExpressions_frag_4();
            true
        }
        "test_expressions_frag_7" => {
            let _ = testExpressions_frag_7();
            true
        }
        "test_expressions_frag_8" => {
            let _ = testExpressions_frag_8();
            true
        }
        "test_expressions_frag_9" => {
            let _ = testExpressions_frag_9();
            true
        }
        "test_expressions_frag_10" => {
            let _ = testExpressions_frag_10();
            true
        }
        "test_expressions_frag_12" => {
            let _ = testExpressions_frag_12();
            true
        }
        "test_expressions_frag_13" => {
            let _ = testExpressions_frag_13();
            true
        }
        "test_expressions_frag_14" => {
            let _ = testExpressions_frag_14();
            true
        }
        "test_expressions_frag_15" => {
            let _ = testExpressions_frag_15();
            true
        }
        "test_expressions_frag_17" => {
            let _ = testExpressions_frag_17();
            true
        }
        "test_expressions_frag_18" => {
            let _ = testExpressions_frag_18();
            true
        }
        "test_expressions_frag_19" => {
            let _ = testExpressions_frag_19();
            true
        }
        "test_expressions_frag_20" => {
            let _ = testExpressions_frag_20();
            true
        }
        "test_expressions_frag_21" => {
            let _ = testExpressions_frag_21();
            true
        }
        "test_expressions_frag_22" => {
            let _ = testExpressions_frag_22();
            true
        }
        "test_expressions_frag_24" => {
            let _ = testExpressions_frag_24();
            true
        }
        "test_expressions_frag_25" => {
            let _ = testExpressions_frag_25();
            true
        }
        "test_expressions_frag_26" => {
            let _ = testExpressions_frag_26();
            true
        }
        "test_expressions_frag_27" => {
            let _ = testExpressions_frag_27();
            true
        }
        "test_expressions_frag_28" => {
            let _ = testExpressions_frag_28();
            true
        }
        "test_expressions_frag_29" => {
            let _ = testExpressions_frag_29();
            true
        }
        "test_expressions_frag_30" => {
            let _ = testExpressions_frag_30();
            true
        }
        "test_expressions_frag_31" => {
            let _ = testExpressions_frag_31();
            true
        }
        "test_expressions_frag_32" => {
            let _ = testExpressions_frag_32();
            true
        }
        "test_expressions_frag_33" => {
            let _ = testExpressions_frag_33();
            true
        }
        "test_expressions_frag_34" => {
            let _ = testExpressions_frag_34();
            true
        }
        "test_expressions_frag_35" => {
            let _ = testExpressions_frag_35();
            true
        }
        "test_expressions_frag_36" => {
            let _ = testExpressions_frag_36();
            true
        }
        "test_expressions_frag_37" => {
            let _ = testExpressions_frag_37();
            true
        }
        "test_expressions_frag_38" => {
            let _ = testExpressions_frag_38();
            true
        }
        "test_expressions_frag_39" => {
            let _ = testExpressions_frag_39();
            true
        }
        "test_expressions_frag_40" => {
            let _ = testExpressions_frag_40();
            true
        }
        "test_expressions_frag_41" => {
            let _ = testExpressions_frag_41();
            true
        }
        "test_expressions_frag_42" => {
            let _ = testExpressions_frag_42();
            true
        }
        "test_expressions_frag_43" => {
            let _ = testExpressions_frag_43();
            true
        }
        "test_expressions_frag_44" => {
            let _ = testExpressions_frag_44();
            true
        }
        "test_expressions_frag_45" => {
            let _ = testExpressions_frag_45();
            true
        }
        "test_expressions_frag_46" => {
            let _ = testExpressions_frag_46();
            true
        }
        "test_expressions_frag_47" => {
            let _ = testExpressions_frag_47();
            true
        }
        "test_expressions_frag_48" => {
            let _ = testExpressions_frag_48();
            true
        }
        "test_expressions_frag_49" => {
            let _ = testExpressions_frag_49();
            true
        }
        "test_expressions_frag_50" => {
            let _ = testExpressions_frag_50();
            true
        }
        "test_expressions_frag_51" => {
            let _ = testExpressions_frag_51();
            true
        }
        "test_expressions_frag_52" => {
            let _ = testExpressions_frag_52();
            true
        }
        "test_expressions_frag_53" => {
            let _ = testExpressions_frag_53();
            true
        }
        "test_expressions_frag_54" => {
            let _ = testExpressions_frag_54();
            true
        }
        "test_expressions_frag_55" => {
            let _ = testExpressions_frag_55();
            true
        }
        "test_expressions_frag_56" => {
            let _ = testExpressions_frag_56();
            true
        }
        "test_expressions_frag_57" => {
            let _ = testExpressions_frag_57();
            true
        }
        "test_expressions_frag_58" => {
            let _ = testExpressions_frag_58();
            true
        }
        "test_expressions_frag_59" => {
            let _ = testExpressions_frag_59();
            true
        }
        "test_expressions_frag_60" => {
            let _ = testExpressions_frag_60();
            true
        }
        "test_expressions_frag_61" => {
            let _ = testExpressions_frag_61();
            true
        }
        "test_expressions_frag_62" => {
            let _ = testExpressions_frag_62();
            true
        }
        "test_expressions_frag_63" => {
            let _ = testExpressions_frag_63();
            true
        }
        "test_expressions_frag_64" => {
            let _ = testExpressions_frag_64();
            true
        }
        "test_expressions_frag_65" => {
            let _ = testExpressions_frag_65();
            true
        }
        "test_expressions_frag_66" => {
            let _ = testExpressions_frag_66();
            true
        }
        "test_expressions_frag_67" => {
            let _ = testExpressions_frag_67();
            true
        }
        "test_expressions_frag_69" => {
            let _ = testExpressions_frag_69();
            true
        }
        "test_expressions_frag_71" => {
            let _ = testExpressions_frag_71();
            true
        }
        "test_expressions_frag_72" => {
            let _ = testExpressions_frag_72();
            true
        }
        "test_expressions_frag_73" => {
            let _ = testExpressions_frag_73();
            true
        }
        "test_expressions_frag_76" => {
            let _ = testExpressions_frag_76();
            true
        }
        "test_expressions_frag_77" => {
            let _ = testExpressions_frag_77();
            true
        }
        "test_expressions_frag_80" => {
            let _ = testExpressions_frag_80();
            true
        }
        "test_expressions_frag_82" => {
            let _ = testExpressions_frag_82();
            true
        }
        "test_expressions_frag_83" => {
            let _ = testExpressions_frag_83();
            true
        }
        "test_expressions_frag_84" => {
            let _ = testExpressions_frag_84();
            true
        }
        "test_expressions_frag_85" => {
            let _ = testExpressions_frag_85();
            true
        }
        "test_expressions_frag_86" => {
            let _ = testExpressions_frag_86();
            true
        }
        "test_expressions_frag_87" => {
            let _ = testExpressions_frag_87();
            true
        }
        "test_expressions_frag_88" => {
            let _ = testExpressions_frag_88();
            true
        }
        "test_expressions_frag_89" => {
            let _ = testExpressions_frag_89();
            true
        }
        "test_expressions_frag_90" => {
            let _ = testExpressions_frag_90();
            true
        }
        "test_expressions_frag_92" => {
            let _ = testExpressions_frag_92();
            true
        }
        "test_expressions_frag_93" => {
            let _ = testExpressions_frag_93();
            true
        }
        "test_expressions_frag_94" => {
            let _ = testExpressions_frag_94();
            true
        }
        "test_expressions_frag_95" => {
            let _ = testExpressions_frag_95();
            true
        }
        "test_expressions_frag_96" => {
            let _ = testExpressions_frag_96();
            true
        }
        "test_expressions_frag_97" => {
            let _ = testExpressions_frag_97();
            true
        }
        "test_expressions_frag_99" => {
            let _ = testExpressions_frag_99();
            true
        }
        "test_expressions_frag_100" => {
            let _ = testExpressions_frag_100();
            true
        }
        "test_expressions_frag_102" => {
            let _ = testExpressions_frag_102();
            true
        }
        "test_expressions_frag_103" => {
            let _ = testExpressions_frag_103();
            true
        }
        "test_expressions_frag_104" => {
            let _ = testExpressions_frag_104();
            true
        }
        "test_expressions_frag_106" => {
            let _ = testExpressions_frag_106();
            true
        }
        "test_expressions_frag_109" => {
            let _ = testExpressions_frag_109();
            true
        }
        "test_expressions_frag_110" => {
            let _ = testExpressions_frag_110();
            true
        }
        "test_expressions_frag_111" => {
            let _ = testExpressions_frag_111();
            true
        }
        "test_expressions_frag_112" => {
            let _ = testExpressions_frag_112();
            true
        }
        "test_expressions_frag_113" => {
            let _ = testExpressions_frag_113();
            true
        }
        "test_expressions_frag_114" => {
            let _ = testExpressions_frag_114();
            true
        }
        "test_expressions_frag_115" => {
            let _ = testExpressions_frag_115();
            true
        }
        "test_expressions_frag_117" => {
            let _ = testExpressions_frag_117();
            true
        }
        "test_expressions_frag_119" => {
            let _ = testExpressions_frag_119();
            true
        }
        "test_expressions_frag_121" => {
            let _ = testExpressions_frag_121();
            true
        }
        "test_expressions_frag_122" => {
            let _ = testExpressions_frag_122();
            true
        }
        "test_expressions_frag_123" => {
            let _ = testExpressions_frag_123();
            true
        }
        "test_expressions_frag_124" => {
            let _ = testExpressions_frag_124();
            true
        }
        "test_expressions_frag_125" => {
            let _ = testExpressions_frag_125();
            true
        }
        "test_expressions_frag_126" => {
            let _ = testExpressions_frag_126();
            true
        }
        "test_expressions_frag_128" => {
            let _ = testExpressions_frag_128();
            true
        }
        "test_expressions_frag_129" => {
            let _ = testExpressions_frag_129();
            true
        }
        "test_expressions_frag_131" => {
            let _ = testExpressions_frag_131();
            true
        }
        "test_expressions_frag_132" => {
            let _ = testExpressions_frag_132();
            true
        }
        "test_expressions_frag_133" => {
            let _ = testExpressions_frag_133();
            true
        }
        "test_expressions_frag_134" => {
            let _ = testExpressions_frag_134();
            true
        }
        "test_expressions_frag_135" => {
            let _ = testExpressions_frag_135();
            true
        }
        "test_expressions_frag_136" => {
            let _ = testExpressions_frag_136();
            true
        }
        "test_expressions_frag_137" => {
            let _ = testExpressions_frag_137();
            true
        }
        "test_expressions_frag_138" => {
            let _ = testExpressions_frag_138();
            true
        }
        "test_expressions_frag_139" => {
            let _ = testExpressions_frag_139();
            true
        }
        "test_expressions_frag_141" => {
            let _ = testExpressions_frag_141();
            true
        }
        "test_expressions_frag_142" => {
            let _ = testExpressions_frag_142();
            true
        }
        "test_expressions_frag_143" => {
            let _ = testExpressions_frag_143();
            true
        }
        "test_expressions_frag_144" => {
            let _ = testExpressions_frag_144();
            true
        }
        "test_expressions_frag_145" => {
            let _ = testExpressions_frag_145();
            true
        }
        "test_expressions_frag_146" => {
            let _ = testExpressions_frag_146();
            true
        }
        "test_expressions_frag_148" => {
            let _ = testExpressions_frag_148();
            true
        }
        "test_expressions_frag_149" => {
            let _ = testExpressions_frag_149();
            true
        }
        "test_expressions_frag_150" => {
            let _ = testExpressions_frag_150();
            true
        }
        "test_expressions_frag_151" => {
            let _ = testExpressions_frag_151();
            true
        }
        "test_expressions_frag_152" => {
            let _ = testExpressions_frag_152();
            true
        }
        "test_expressions_frag_153" => {
            let _ = testExpressions_frag_153();
            true
        }
        "test_expressions_frag_155" => {
            let _ = testExpressions_frag_155();
            true
        }
        "test_expressions_frag_156" => {
            let _ = testExpressions_frag_156();
            true
        }
        "test_expressions_frag_157" => {
            let _ = testExpressions_frag_157();
            true
        }
        "test_expressions_frag_158" => {
            let _ = testExpressions_frag_158();
            true
        }
        "test_expressions_frag_159" => {
            let _ = testExpressions_frag_159();
            true
        }
        "test_expressions_frag_160" => {
            let _ = testExpressions_frag_160();
            true
        }
        "test_builtins_frag_0" => {
            let _ = testBuiltins_frag_0();
            true
        }
        "test_builtins_frag_3" => {
            let _ = testBuiltins_frag_3();
            true
        }
        "test_builtins_frag_8" => {
            let _ = testBuiltins_frag_8();
            true
        }
        "test_builtins_frag_16" => {
            let _ = testBuiltins_frag_16();
            true
        }
        "test_builtins_frag_17" => {
            let _ = testBuiltins_frag_17();
            true
        }
        "test_builtins_frag_18" => {
            let _ = testBuiltins_frag_18();
            true
        }
        "test_builtins_frag_32" => {
            let _ = testBuiltins_frag_32();
            true
        }
        "test_builtins_frag_34" => {
            let _ = testBuiltins_frag_34();
            true
        }
        "test_builtins_frag_35" => {
            let _ = testBuiltins_frag_35();
            true
        }
        "test_builtins_frag_37" => {
            let _ = testBuiltins_frag_37();
            true
        }
        "test_builtins_frag_39" => {
            let _ = testBuiltins_frag_39();
            true
        }
        "test_builtins_frag_40" => {
            let _ = testBuiltins_frag_40();
            true
        }
        "test_builtins_frag_41" => {
            let _ = testBuiltins_frag_41();
            true
        }
        "test_builtins_frag_43" => {
            let _ = testBuiltins_frag_43();
            true
        }
        "test_builtins_frag_44" => {
            let _ = testBuiltins_frag_44();
            true
        }
        "test_builtins_frag_46" => {
            let _ = testBuiltins_frag_46();
            true
        }
        "test_builtins_frag_48" => {
            let _ = testBuiltins_frag_48();
            true
        }
        "test_builtins_frag_49" => {
            let _ = testBuiltins_frag_49();
            true
        }
        "test_builtins_frag_50" => {
            let _ = testBuiltins_frag_50();
            true
        }
        "test_builtins_frag_51" => {
            let _ = testBuiltins_frag_51();
            true
        }
        "test_builtins_frag_52" => {
            let _ = testBuiltins_frag_52();
            true
        }
        "test_builtins_frag_53" => {
            let _ = testBuiltins_frag_53();
            true
        }
        "test_builtins_frag_54" => {
            let _ = testBuiltins_frag_54();
            true
        }
        "test_builtins_frag_55" => {
            let _ = testBuiltins_frag_55();
            true
        }
        "test_builtins_frag_56" => {
            let _ = testBuiltins_frag_56();
            true
        }
        "test_builtins_frag_57" => {
            let _ = testBuiltins_frag_57();
            true
        }
        "test_builtins_frag_60" => {
            let _ = testBuiltins_frag_60();
            true
        }
        "test_builtins_frag_61" => {
            let _ = testBuiltins_frag_61();
            true
        }
        "test_builtins_frag_68" => {
            let _ = testBuiltins_frag_68();
            true
        }
        "test_builtins_frag_71" => {
            let _ = testBuiltins_frag_71();
            true
        }
        "test_builtins_frag_74" => {
            let _ = testBuiltins_frag_74();
            true
        }
        "test_builtins_frag_80" => {
            let _ = testBuiltins_frag_80();
            true
        }
        "test_builtins_frag_81" => {
            let _ = testBuiltins_frag_81();
            true
        }
        "test_builtins_frag_95" => {
            let _ = testBuiltins_frag_95();
            true
        }
        "test_builtins_frag_96" => {
            let _ = testBuiltins_frag_96();
            true
        }
        "test_builtins_frag_97" => {
            let _ = testBuiltins_frag_97();
            true
        }
        "test_builtins_frag_98" => {
            let _ = testBuiltins_frag_98();
            true
        }
        "test_builtins_frag_100" => {
            let _ = testBuiltins_frag_100();
            true
        }
        "test_builtins_frag_101" => {
            let _ = testBuiltins_frag_101();
            true
        }
        "test_builtins_frag_102" => {
            let _ = testBuiltins_frag_102();
            true
        }
        "test_builtins_frag_103" => {
            let _ = testBuiltins_frag_103();
            true
        }
        "test_builtins_frag_104" => {
            let _ = testBuiltins_frag_104();
            true
        }
        "test_builtins_frag_105" => {
            let _ = testBuiltins_frag_105();
            true
        }
        "test_builtins_frag_106" => {
            let _ = testBuiltins_frag_106();
            true
        }
        "test_builtins_frag_107" => {
            let _ = testBuiltins_frag_107();
            true
        }
        "test_builtins_frag_110" => {
            let _ = testBuiltins_frag_110();
            true
        }
        "test_builtins_frag_113" => {
            let _ = testBuiltins_frag_113();
            true
        }
        "test_builtins_frag_115" => {
            let _ = testBuiltins_frag_115();
            true
        }
        "test_builtins_frag_116" => {
            let _ = testBuiltins_frag_116();
            true
        }
        "test_builtins_frag_126" => {
            let _ = testBuiltins_frag_126();
            true
        }
        "test_builtins_frag_130" => {
            let _ = testBuiltins_frag_130();
            true
        }
        "test_builtins_frag_137" => {
            let _ = testBuiltins_frag_137();
            true
        }
        "test_builtins_frag_150" => {
            let _ = testBuiltins_frag_150();
            true
        }
        "test_builtins_frag_151" => {
            let _ = testBuiltins_frag_151();
            true
        }
        "test_builtins_frag_152" => {
            let _ = testBuiltins_frag_152();
            true
        }
        "test_builtins_frag_153" => {
            let _ = testBuiltins_frag_153();
            true
        }
        "test_builtins_frag_154" => {
            let _ = testBuiltins_frag_154();
            true
        }
        "test_builtins_frag_156" => {
            let _ = testBuiltins_frag_156();
            true
        }
        "test_builtins_frag_157" => {
            let _ = testBuiltins_frag_157();
            true
        }
        "test_builtins_frag_158" => {
            let _ = testBuiltins_frag_158();
            true
        }
        "test_builtins_frag_159" => {
            let _ = testBuiltins_frag_159();
            true
        }
        "test_builtins_frag_160" => {
            let _ = testBuiltins_frag_160();
            true
        }
        "test_builtins_frag_161" => {
            let _ = testBuiltins_frag_161();
            true
        }
        "test_builtins_frag_162" => {
            let _ = testBuiltins_frag_162();
            true
        }
        "test_builtins_frag_163" => {
            let _ = testBuiltins_frag_163();
            true
        }
        "test_builtins_frag_164" => {
            let _ = testBuiltins_frag_164();
            true
        }
        "test_builtins_frag_165" => {
            let _ = testBuiltins_frag_165();
            true
        }
        "test_builtins_frag_166" => {
            let _ = testBuiltins_frag_166();
            true
        }
        "test_builtins_frag_167" => {
            let _ = testBuiltins_frag_167();
            true
        }
        "test_builtins_frag_168" => {
            let _ = testBuiltins_frag_168();
            true
        }
        "test_builtins_frag_169" => {
            let _ = testBuiltins_frag_169();
            true
        }
        "test_builtins_frag_170" => {
            let _ = testBuiltins_frag_170();
            true
        }
        "test_builtins_frag_172" => {
            let _ = testBuiltins_frag_172();
            true
        }
        "test_builtins_frag_175" => {
            let _ = testBuiltins_frag_175();
            true
        }
        "test_builtins_frag_176" => {
            let _ = testBuiltins_frag_176();
            true
        }
        "test_builtins_frag_177" => {
            let _ = testBuiltins_frag_177();
            true
        }
        "test_builtins_frag_179" => {
            let _ = testBuiltins_frag_179();
            true
        }
        "test_builtins_frag_182" => {
            let _ = testBuiltins_frag_182();
            true
        }
        "test_builtins_frag_202" => {
            let _ = testBuiltins_frag_202();
            true
        }
        "test_builtins_frag_204" => {
            let _ = testBuiltins_frag_204();
            true
        }
        "test_builtins_frag_205" => {
            let _ = testBuiltins_frag_205();
            true
        }
        "test_builtins_frag_209" => {
            let _ = testBuiltins_frag_209();
            true
        }
        "test_builtins_frag_210" => {
            let _ = testBuiltins_frag_210();
            true
        }
        "test_builtins_frag_211" => {
            let _ = testBuiltins_frag_211();
            true
        }
        "test_builtins_frag_212" => {
            let _ = testBuiltins_frag_212();
            true
        }
        "test_builtins_frag_213" => {
            let _ = testBuiltins_frag_213();
            true
        }
        "test_builtins_frag_214" => {
            let _ = testBuiltins_frag_214();
            true
        }
        "test_builtins_frag_215" => {
            let _ = testBuiltins_frag_215();
            true
        }
        "test_builtins_frag_216" => {
            let _ = testBuiltins_frag_216();
            true
        }
        "test_builtins_frag_218" => {
            let _ = testBuiltins_frag_218();
            true
        }
        "test_builtins_frag_219" => {
            let _ = testBuiltins_frag_219();
            true
        }
        "test_builtins_frag_223" => {
            let _ = testBuiltins_frag_223();
            true
        }
        "test_builtins_frag_224" => {
            let _ = testBuiltins_frag_224();
            true
        }
        "test_builtins_frag_225" => {
            let _ = testBuiltins_frag_225();
            true
        }
        "test_builtins_frag_226" => {
            let _ = testBuiltins_frag_226();
            true
        }
        "test_builtins_frag_227" => {
            let _ = testBuiltins_frag_227();
            true
        }
        "test_builtins_frag_228" => {
            let _ = testBuiltins_frag_228();
            true
        }
        _ => false,
    }
}
fn normalize_for_comparison(output: &str) -> String {
    output
        .lines()
        .filter(|line| {
            // Skip stack trace frames: "    at function_name (file:line:col)"
            let trimmed = line.trim_start();
            !trimmed.starts_with("at ")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

/// Run all fragments via child processes (for crash isolation), compare with Node.js output.
fn run_all(binary: &str) {
    let total = ALL_FRAGMENTS.len();
    let mut passed = 0usize;
    let mut mismatched = 0usize;
    let mut errors = 0usize;
    let mut mismatches: Vec<(&str, String, String)> = Vec::new();

    // Check if node is available once up front.
    let node_available = Command::new("node").arg("--version").output().is_ok();
    if !node_available {
        eprintln!("Warning: node not found on PATH — skipping comparison, exit-code only mode.\n");
    }

    for (i, frag) in ALL_FRAGMENTS.iter().enumerate() {
        // Run Zig binary as child process (crash isolation).
        // Note: Zig runtime uses std.debug.print which writes to stderr.
        let zig_result = Command::new(binary).arg(frag).output();

        let zig_output = match zig_result {
            Ok(out) => {
                if !out.status.success() {
                    // Zig panic or non-zero exit.
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    errors += 1;
                    eprintln!("[{}/{}] {} ... CRASH ({})", i + 1, total, frag, stderr.lines().next().unwrap_or("unknown"));
                    continue;
                }
                // console.log goes to stderr via std.debug.print; stdout may also have output.
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.is_empty() {
                    stderr.to_string()
                } else {
                    stdout.to_string()
                }
            }
            Err(e) => {
                errors += 1;
                eprintln!("[{}/{}] {} ... ERROR (spawn): {}", i + 1, total, frag, e);
                continue;
            }
        };

        if !node_available {
            passed += 1;
            eprintln!("OK (no comparison)");
            continue;
        }

        // Run Node.js reference.
        let node_path = format!("{}/{}.node.js", JS_SRC_DIR, frag);
        let node_result = Command::new("node").arg(&node_path).output();

        let node_output = match node_result {
            Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
            Err(_) => {
                // Node.js file might not exist — treat as pass (Zig ran fine).
                passed += 1;
                eprintln!("OK (no node.js reference)");
                continue;
            }
        };

        // Normalize outputs before comparison:
        // - Strip Node.js stack trace lines ("    at ...") since the Zig runtime
        //   does not produce JS-style stack traces.
        // - Trim trailing whitespace/newlines.
        let zig_norm = normalize_for_comparison(&zig_output);
        let node_norm = normalize_for_comparison(&node_output);

        // Compare (trim trailing whitespace/newlines).
        if zig_norm == node_norm {
            passed += 1;
        } else {
            mismatched += 1;
            eprintln!("[{}/{}] {} ... MISMATCH", i + 1, total, frag);
            mismatches.push((frag, node_output, zig_output));
        }
    }

    // Summary.
    eprintln!();
    eprintln!("=== Summary ===");
    eprintln!(
        "Total: {}, Passed: {}, Mismatched: {}, Errors: {}",
        total, passed, mismatched, errors
    );

    if !mismatches.is_empty() {
        eprintln!();
        eprintln!("=== Mismatches ===");
        for (frag, expected, actual) in &mismatches {
            eprintln!();
            eprintln!("  {}:", frag);
            // Show first 3 lines of each for readability.
            let exp_lines: Vec<&str> = expected.trim_end().lines().collect();
            let act_lines: Vec<&str> = actual.trim_end().lines().collect();
            let exp_preview = exp_lines
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n    ");
            let act_preview = act_lines
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n    ");
            eprintln!("    expected ({} lines):", exp_lines.len());
            eprintln!("    {}", exp_preview);
            if exp_lines.len() > 3 {
                eprintln!("    ... ({} more lines)", exp_lines.len() - 3);
            }
            eprintln!("    actual ({} lines):", act_lines.len());
            eprintln!("    {}", act_preview);
            if act_lines.len() > 3 {
                eprintln!("    ... ({} more lines)", act_lines.len() - 3);
            }
        }
    }

    // Exit code 0 regardless of mismatch/error count — mdn-test-project is a
    // diagnostic tool, not a gate. Mismatch details are already printed above.
}
