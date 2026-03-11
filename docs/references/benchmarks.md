# Benchmark References

## Active Local Benchmark Vendors

- SWE-bench Verified
  - local dataset snapshot: `/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/swebench-verified/verified-test.jsonl`
  - upstream dataset: [princeton-nlp/SWE-bench_Verified](https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified)
  - role in this bench: real-repo patching under strong regression verification

- NL2RepoBench
  - local vendor path: `/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/NL2RepoBench`
  - upstream: [NL2RepoBench](https://github.com/THUDM/NL2RepoBench)
  - role in this bench: end-to-end repository delivery and zero-to-one software engineering

- NewtonBench
  - local vendor path: `/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/NewtonBench`
  - upstream: [NewtonBench](https://github.com/HKUST-KnowComp/NewtonBench)
  - role in this bench: interactive scientific discovery and experimentation

## Why These Three Together

- SWE-bench stresses repository debugging and patch verification.
- NL2RepoBench stresses full-repository construction from natural-language specifications.
- NewtonBench stresses exploratory experimentation, law induction, and multi-step hypothesis refinement.

Together they give the Codex research bench three complementary observation regimes:

- patching in existing codebases
- building complete systems from scratch
- discovering latent rules in interactive environments

## Adapter View

Current adapter names in the bench:

- `swebench`
- `nl2repo`
- `newtonbench`
- `repo-patch-jsonl` as the reusable generic lane

This means the benchmark docs should be read not just as benchmark references, but as the benchmark side of a reusable adapter system.
