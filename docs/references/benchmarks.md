# Benchmark References

## Active Local Benchmark Vendors

- NL2RepoBench
  - local vendor path: `/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/NL2RepoBench`
  - upstream: [NL2RepoBench](https://github.com/THUDM/NL2RepoBench)
  - role in this bench: end-to-end repository delivery and zero-to-one software engineering

- NewtonBench
  - local vendor path: `/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/NewtonBench`
  - upstream: [NewtonBench](https://github.com/HKUST-KnowComp/NewtonBench)
  - role in this bench: interactive scientific discovery and experimentation

- SWE-bench Verified
  - upstream dataset: [princeton-nlp/SWE-bench_Verified](https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified)
  - role in this bench: real-repo patching under strong regression verification

## Why These Three Together

- SWE-bench stresses repository debugging and patch verification.
- NL2RepoBench stresses full-repository construction from natural-language specifications.
- NewtonBench stresses exploratory experimentation, law induction, and multi-step hypothesis refinement.

Together they give the Codex research bench three complementary observation regimes:

- patching in existing codebases
- building complete systems from scratch
- discovering latent rules in interactive environments
