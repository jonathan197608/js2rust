// src/main.rs -- CLI dispatcher: ./mdn-test-project <fragment_name>
use js2rust_bridge::js2rust_bridge;
use std::env;

js2rust_bridge!();

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <fragment_name>", args[0]);
        std::process::exit(1);
    }
    let frag = &args[1];
    js2rust_init();
    match frag.as_str() {
        "test_statements_frag_0" => {
            testStatements_frag_0_app();
        }
        "test_statements_frag_11" => {
            let _ = testStatements_frag_11_app();
        }
        "test_statements_frag_12" => {
            testStatements_frag_12_app();
        }
        "test_statements_frag_18" => {
            testStatements_frag_18_app();
        }
        "test_statements_frag_27" => {
            testStatements_frag_27_app();
        }
        "test_statements_frag_3" => {
            testStatements_frag_3_app();
        }
        "test_statements_frag_37" => {
            testStatements_frag_37_app();
        }
        "test_expressions_frag_100" => {
            testExpressions_frag_100_app();
        }
        "test_expressions_frag_102" => {
            testExpressions_frag_102_app();
        }
        "test_expressions_frag_103" => {
            testExpressions_frag_103_app();
        }
        "test_expressions_frag_104" => {
            testExpressions_frag_104_app();
        }
        "test_expressions_frag_109" => {
            testExpressions_frag_109_app();
        }
        "test_expressions_frag_110" => {
            testExpressions_frag_110_app();
        }
        "test_expressions_frag_111" => {
            testExpressions_frag_111_app();
        }
        "test_expressions_frag_112" => {
            testExpressions_frag_112_app();
        }
        "test_expressions_frag_114" => {
            testExpressions_frag_114_app();
        }
        "test_expressions_frag_115" => {
            testExpressions_frag_115_app();
        }
        "test_expressions_frag_117" => {
            testExpressions_frag_117_app();
        }
        "test_expressions_frag_12" => {
            testExpressions_frag_12_app();
        }
        "test_expressions_frag_121" => {
            testExpressions_frag_121_app();
        }
        "test_expressions_frag_122" => {
            testExpressions_frag_122_app();
        }
        "test_expressions_frag_123" => {
            testExpressions_frag_123_app();
        }
        "test_expressions_frag_124" => {
            testExpressions_frag_124_app();
        }
        "test_expressions_frag_125" => {
            testExpressions_frag_125_app();
        }
        "test_expressions_frag_126" => {
            testExpressions_frag_126_app();
        }
        "test_expressions_frag_129" => {
            testExpressions_frag_129_app();
        }
        "test_expressions_frag_131" => {
            testExpressions_frag_131_app();
        }
        "test_expressions_frag_132" => {
            testExpressions_frag_132_app();
        }
        "test_expressions_frag_133" => {
            testExpressions_frag_133_app();
        }
        "test_expressions_frag_134" => {
            testExpressions_frag_134_app();
        }
        "test_expressions_frag_137" => {
            testExpressions_frag_137_app();
        }
        "test_expressions_frag_138" => {
            testExpressions_frag_138_app();
        }
        "test_expressions_frag_139" => {
            testExpressions_frag_139_app();
        }
        "test_expressions_frag_14" => {
            testExpressions_frag_14_app();
        }
        "test_expressions_frag_141" => {
            testExpressions_frag_141_app();
        }
        "test_expressions_frag_144" => {
            testExpressions_frag_144_app();
        }
        "test_expressions_frag_145" => {
            testExpressions_frag_145_app();
        }
        "test_expressions_frag_146" => {
            testExpressions_frag_146_app();
        }
        "test_expressions_frag_148" => {
            testExpressions_frag_148_app();
        }
        "test_expressions_frag_15" => {
            testExpressions_frag_15_app();
        }
        "test_expressions_frag_151" => {
            testExpressions_frag_151_app();
        }
        "test_expressions_frag_152" => {
            testExpressions_frag_152_app();
        }
        "test_expressions_frag_153" => {
            testExpressions_frag_153_app();
        }
        "test_expressions_frag_155" => {
            testExpressions_frag_155_app();
        }
        "test_expressions_frag_158" => {
            testExpressions_frag_158_app();
        }
        "test_expressions_frag_159" => {
            testExpressions_frag_159_app();
        }
        "test_expressions_frag_160" => {
            testExpressions_frag_160_app();
        }
        "test_expressions_frag_17" => {
            testExpressions_frag_17_app();
        }
        "test_expressions_frag_2" => {
            testExpressions_frag_2_app();
        }
        "test_expressions_frag_20" => {
            testExpressions_frag_20_app();
        }
        "test_expressions_frag_21" => {
            testExpressions_frag_21_app();
        }
        "test_expressions_frag_22" => {
            testExpressions_frag_22_app();
        }
        "test_expressions_frag_24" => {
            testExpressions_frag_24_app();
        }
        "test_expressions_frag_26" => {
            testExpressions_frag_26_app();
        }
        "test_expressions_frag_27" => {
            testExpressions_frag_27_app();
        }
        "test_expressions_frag_28" => {
            testExpressions_frag_28_app();
        }
        "test_expressions_frag_3" => {
            testExpressions_frag_3_app();
        }
        "test_expressions_frag_30" => {
            testExpressions_frag_30_app();
        }
        "test_expressions_frag_31" => {
            testExpressions_frag_31_app();
        }
        "test_expressions_frag_32" => {
            testExpressions_frag_32_app();
        }
        "test_expressions_frag_34" => {
            testExpressions_frag_34_app();
        }
        "test_expressions_frag_35" => {
            testExpressions_frag_35_app();
        }
        "test_expressions_frag_36" => {
            testExpressions_frag_36_app();
        }
        "test_expressions_frag_37" => {
            testExpressions_frag_37_app();
        }
        "test_expressions_frag_39" => {
            testExpressions_frag_39_app();
        }
        "test_expressions_frag_4" => {
            testExpressions_frag_4_app();
        }
        "test_expressions_frag_40" => {
            testExpressions_frag_40_app();
        }
        "test_expressions_frag_41" => {
            testExpressions_frag_41_app();
        }
        "test_expressions_frag_43" => {
            testExpressions_frag_43_app();
        }
        "test_expressions_frag_44" => {
            testExpressions_frag_44_app();
        }
        "test_expressions_frag_45" => {
            testExpressions_frag_45_app();
        }
        "test_expressions_frag_47" => {
            testExpressions_frag_47_app();
        }
        "test_expressions_frag_48" => {
            testExpressions_frag_48_app();
        }
        "test_expressions_frag_49" => {
            testExpressions_frag_49_app();
        }
        "test_expressions_frag_50" => {
            testExpressions_frag_50_app();
        }
        "test_expressions_frag_51" => {
            testExpressions_frag_51_app();
        }
        "test_expressions_frag_58" => {
            testExpressions_frag_58_app();
        }
        "test_expressions_frag_60" => {
            testExpressions_frag_60_app();
        }
        "test_expressions_frag_61" => {
            testExpressions_frag_61_app();
        }
        "test_expressions_frag_7" => {
            testExpressions_frag_7_app();
        }
        "test_expressions_frag_72" => {
            testExpressions_frag_72_app();
        }
        "test_expressions_frag_77" => {
            testExpressions_frag_77_app();
        }
        "test_expressions_frag_80" => {
            testExpressions_frag_80_app();
        }
        "test_expressions_frag_82" => {
            testExpressions_frag_82_app();
        }
        "test_expressions_frag_83" => {
            testExpressions_frag_83_app();
        }
        "test_expressions_frag_85" => {
            testExpressions_frag_85_app();
        }
        "test_expressions_frag_88" => {
            testExpressions_frag_88_app();
        }
        "test_expressions_frag_9" => {
            testExpressions_frag_9_app();
        }
        "test_expressions_frag_90" => {
            testExpressions_frag_90_app();
        }
        "test_expressions_frag_93" => {
            testExpressions_frag_93_app();
        }
        "test_expressions_frag_94" => {
            testExpressions_frag_94_app();
        }
        "test_expressions_frag_95" => {
            testExpressions_frag_95_app();
        }
        "test_expressions_frag_96" => {
            testExpressions_frag_96_app();
        }
        "test_expressions_frag_97" => {
            testExpressions_frag_97_app();
        }
        "test_builtins_frag_0" => {
            testBuiltins_frag_0_app();
        }
        "test_builtins_frag_100" => {
            testBuiltins_frag_100_app();
        }
        "test_builtins_frag_101" => {
            testBuiltins_frag_101_app();
        }
        "test_builtins_frag_102" => {
            testBuiltins_frag_102_app();
        }
        "test_builtins_frag_104" => {
            testBuiltins_frag_104_app();
        }
        "test_builtins_frag_115" => {
            testBuiltins_frag_115_app();
        }
        "test_builtins_frag_116" => {
            testBuiltins_frag_116_app();
        }
        "test_builtins_frag_126" => {
            testBuiltins_frag_126_app();
        }
        "test_builtins_frag_130" => {
            testBuiltins_frag_130_app();
        }
        "test_builtins_frag_150" => {
            testBuiltins_frag_150_app();
        }
        "test_builtins_frag_151" => {
            testBuiltins_frag_151_app();
        }
        "test_builtins_frag_152" => {
            testBuiltins_frag_152_app();
        }
        "test_builtins_frag_153" => {
            testBuiltins_frag_153_app();
        }
        "test_builtins_frag_154" => {
            testBuiltins_frag_154_app();
        }
        "test_builtins_frag_157" => {
            testBuiltins_frag_157_app();
        }
        "test_builtins_frag_159" => {
            testBuiltins_frag_159_app();
        }
        "test_builtins_frag_160" => {
            testBuiltins_frag_160_app();
        }
        "test_builtins_frag_161" => {
            testBuiltins_frag_161_app();
        }
        "test_builtins_frag_162" => {
            testBuiltins_frag_162_app();
        }
        "test_builtins_frag_163" => {
            testBuiltins_frag_163_app();
        }
        "test_builtins_frag_165" => {
            testBuiltins_frag_165_app();
        }
        "test_builtins_frag_166" => {
            testBuiltins_frag_166_app();
        }
        "test_builtins_frag_17" => {
            testBuiltins_frag_17_app();
        }
        "test_builtins_frag_170" => {
            testBuiltins_frag_170_app();
        }
        "test_builtins_frag_176" => {
            testBuiltins_frag_176_app();
        }
        "test_builtins_frag_177" => {
            testBuiltins_frag_177_app();
        }
        "test_builtins_frag_182" => {
            testBuiltins_frag_182_app();
        }
        "test_builtins_frag_202" => {
            let _ = testBuiltins_frag_202_app();
        }
        "test_builtins_frag_204" => {
            testBuiltins_frag_204_app();
        }
        "test_builtins_frag_205" => {
            testBuiltins_frag_205_app();
        }
        "test_builtins_frag_210" => {
            testBuiltins_frag_210_app();
        }
        "test_builtins_frag_215" => {
            testBuiltins_frag_215_app();
        }
        "test_builtins_frag_216" => {
            testBuiltins_frag_216_app();
        }
        "test_builtins_frag_218" => {
            testBuiltins_frag_218_app();
        }
        "test_builtins_frag_3" => {
            testBuiltins_frag_3_app();
        }
        "test_builtins_frag_32" => {
            testBuiltins_frag_32_app();
        }
        "test_builtins_frag_34" => {
            testBuiltins_frag_34_app();
        }
        "test_builtins_frag_35" => {
            testBuiltins_frag_35_app();
        }
        "test_builtins_frag_37" => {
            testBuiltins_frag_37_app();
        }
        "test_builtins_frag_39" => {
            testBuiltins_frag_39_app();
        }
        "test_builtins_frag_40" => {
            testBuiltins_frag_40_app();
        }
        "test_builtins_frag_41" => {
            testBuiltins_frag_41_app();
        }
        "test_builtins_frag_43" => {
            testBuiltins_frag_43_app();
        }
        "test_builtins_frag_44" => {
            testBuiltins_frag_44_app();
        }
        "test_builtins_frag_46" => {
            testBuiltins_frag_46_app();
        }
        "test_builtins_frag_48" => {
            testBuiltins_frag_48_app();
        }
        "test_builtins_frag_49" => {
            testBuiltins_frag_49_app();
        }
        "test_builtins_frag_50" => {
            testBuiltins_frag_50_app();
        }
        "test_builtins_frag_51" => {
            testBuiltins_frag_51_app();
        }
        "test_builtins_frag_52" => {
            testBuiltins_frag_52_app();
        }
        "test_builtins_frag_53" => {
            testBuiltins_frag_53_app();
        }
        "test_builtins_frag_54" => {
            testBuiltins_frag_54_app();
        }
        "test_builtins_frag_55" => {
            testBuiltins_frag_55_app();
        }
        "test_builtins_frag_56" => {
            testBuiltins_frag_56_app();
        }
        "test_builtins_frag_57" => {
            testBuiltins_frag_57_app();
        }
        "test_builtins_frag_74" => {
            testBuiltins_frag_74_app();
        }
        "test_builtins_frag_8" => {
            testBuiltins_frag_8_app();
        }
        "test_builtins_frag_80" => {
            testBuiltins_frag_80_app();
        }
        "test_builtins_frag_81" => {
            testBuiltins_frag_81_app();
        }
        _ => {
            eprintln!("Unknown fragment: {}", frag);
            std::process::exit(1);
        }
    }
    js2rust_deinit();
}
