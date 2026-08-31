# AIP Local Multimodal AI Roadmap

Status: **ROADMAP / ARCHITECTURE TARGET — NOT IMPLEMENTED**

This document defines the planned local multimodal AI architecture for AIP. It is intentionally separate from current implementation status and must not be read as a release claim.

## Reference hardware

The architecture must be practical on the reference desktop rather than designed around high-end datacenter or 24 GB-class GPUs:

- NVIDIA RTX 3070 Ti
- 8 GB VRAM
- 32 GB system RAM
- Windows
- CUDA

The design must remain useful under this constraint through quantization, model lifecycle management, CPU/RAM placement, bounded concurrency, and loading specialized models only when needed.

## Architectural principle

AIP should not depend on one giant Any-to-Any model.

The central LLM remains the agent's reasoning and orchestration brain. Other models are specialized local tools coordinated by AIP.

Target composition:

`LLM + embeddings + reranker + VAD + ASR + TTS + VLM + image embeddings + object detection`

Capabilities should share models where that reduces duplication without materially reducing quality.

## Consolidated capability set

The following capabilities are in scope:

| Capability | Role in AIP | Priority | Notes |
| --- | --- | --- | --- |
| Text Generation / LLM | Main agent reasoning, conversation, tool planning and coding | Essential | Central brain; current Qwen-family 7B–9B-class quantized models are initial candidates, not final choices. |
| Text Feature Extraction / Embeddings | Semantic representation for memory, RAG and documents | Essential | Prefer one strong multilingual embedding model shared across memory, RAG and similarity. |
| Sentence Similarity | Semantic retrieval and matching | Essential | Should normally use the embedding model rather than a separate dedicated model. |
| Text Ranking / Reranking | Reorder retrieved context before the LLM | High | Small multilingual reranker loaded on demand or kept in CPU/RAM when practical. |
| Automatic Speech Recognition | Speech-to-text for agent listening | Essential | PT-BR quality is a primary acceptance criterion. |
| Text-to-Speech | Agent voice output | Essential | PT-BR, latency and future voice identity/cloning support matter. |
| Voice Activity Detection | Detect speech start/stop | Essential | Prefer a tiny CPU-resident model. |
| Image-Text-to-Text / VLM | Screenshots, photos, UI understanding and contextual OCR | High | Main visual reasoning model. |
| Visual Question Answering | Questions about visual content | High | Do not add a separate model when the selected VLM already handles VQA adequately. |
| Document Question Answering | Ask questions over documents | High | Prefer a RAG pipeline, not a dedicated Document-QA model by default. |
| Image Feature Extraction | Visual memory, image similarity and retrieval | High | Shared visual embedding model such as SigLIP/CLIP-class architecture or a better benchmarked successor. |
| Object Detection | Locate screen/image objects with positions and confidence | Medium | Load only when spatial detection is actually required. |

## Explicitly outside this roadmap

The following are intentionally excluded unless a later product need justifies reopening them:

- Text-to-Image;
- Image-to-Image;
- image generation/editing models;
- video generation/editing;
- Text-to-3D;
- Image-to-3D;
- Keypoint Detection / pose estimation;
- other model categories without direct agent utility.

## Shared-model consolidation

The roadmap should avoid artificial one-capability-one-model mapping.

### Embeddings + similarity

One multilingual text embedding model should ideally serve:

- long-term memory retrieval;
- RAG;
- sentence similarity;
- document chunk retrieval;
- semantic search.

A separate sentence-similarity model is not required unless benchmarks prove a meaningful advantage.

### VLM + VQA

The selected VLM should serve:

- screenshot understanding;
- photo understanding;
- UI interpretation;
- contextual OCR;
- description;
- VQA.

A dedicated VQA model should only be added if the VLM demonstrably cannot satisfy the AIP workload.

### Document QA

Default architecture:

`document -> parser/chunking -> embeddings -> vector retrieval -> reranker -> context -> LLM`

For visually complex PDFs or pages:

`page/image -> VLM -> structured/contextual extraction -> RAG/LLM`

Do not introduce a dedicated Document-QA model without benchmark evidence that the composed pipeline is insufficient.

## Target processing flows

### Voice

`Microphone -> VAD -> ASR -> agent/LLM -> TTS -> audio output`

VAD should remain lightweight and may stay CPU-resident. ASR/TTS may use GPU opportunistically but must release resources according to lifecycle policy.

### Memory and RAG

`information -> embeddings -> vector index -> similarity retrieval -> reranker -> selected context -> LLM`

The LLM must not be responsible for scanning the entire durable memory store on every request.

### Vision

`image/screenshot -> VLM -> contextual representation -> agent/LLM`

When spatial coordinates or deterministic object locations are required:

`image/screenshot -> object detector -> objects + positions + confidence -> agent`

### Visual memory

`image -> image embeddings -> local index -> future similarity retrieval`

Visual embeddings should not imply permanent screenshot retention. Storage/privacy rules remain separate product decisions.

### Documents

`document -> parsing/chunking -> text embeddings -> retrieval -> reranking -> LLM`

VLM participates only for pages whose information cannot be reliably represented by ordinary parsing alone.

## Model/resource manager foundation

Before expanding the model fleet, AIP needs a first-class model and resource lifecycle layer.

It should eventually own:

- installed-model/provider inventory;
- model capability metadata;
- CPU/GPU placement;
- VRAM budgets;
- RAM budgets;
- quantization metadata;
- model load/unload lifecycle;
- idle eviction;
- priority and queueing;
- cancellation;
- one-heavy-model-at-a-time policy where necessary;
- provider health;
- warm/resident small models;
- on-demand heavy models;
- fallback/degraded states;
- hardware capability detection;
- benchmark-derived profiles.

The resource manager must remain deterministic and observable. The LLM may request a capability, but it should not directly control unsafe resource/process behavior.

## 8 GB VRAM operating strategy

The roadmap assumes models will not all remain resident on the GPU simultaneously.

Target behavior:

1. keep the primary LLM active when possible;
2. keep tiny components such as VAD on CPU/RAM;
3. use CPU/RAM for embeddings or reranking when latency remains acceptable;
4. when a heavy visual/audio model is required, reserve resources before loading it;
5. temporarily unload or offload another heavy model when needed;
6. execute the specialized capability;
7. release VRAM after the operation or after a short configurable warm period;
8. restore/warm the primary conversational path as needed.

Example:

`LLM active -> vision requested -> resource manager reserves VRAM -> VLM loads -> analysis completes -> VLM unloads -> conversational LLM continues`

The exact placement policy must be benchmarked. The roadmap must not assume simultaneous residency of an 8B-class LLM, large ASR, VLM and TTS inside 8 GB.

## Initial model candidate families

These are benchmark candidates, not architecture commitments.

### LLM

- current Qwen-family model in roughly the 7B–9B class, quantized;
- alternatives should be benchmarked for PT-BR, reasoning, coding, tool calling, memory use and usable context within the 8 GB target.

### Text embeddings

- BGE-M3;
- multilingual E5-family models;
- modern multilingual BGE alternatives;
- any newer model that materially wins AIP-specific retrieval benchmarks.

### Reranker

- multilingual BGE reranker-class models;
- smaller alternatives if CPU latency is materially better.

### VAD

- Silero VAD-class model or an equivalent tiny local detector.

### ASR

- Whisper Large-v3 Turbo through Faster-Whisper/CTranslate2 as a quality candidate;
- smaller/faster alternatives must be benchmarked for PT-BR latency and accuracy on the reference PC.

### TTS

- Qwen3-TTS 0.6B-class candidate;
- equivalent local TTS alternatives should be compared for PT-BR quality, latency, VRAM, and future voice-identity support.

### VLM

- current Qwen-VL/Qwen multimodal model in approximately the 3B–4B class, quantized;
- equivalent compact VLMs should be benchmarked on screenshots, interfaces, OCR context, photographs and VQA.

### Image embeddings

- SigLIP-class models;
- CLIP-class models;
- newer local alternatives if they improve visual retrieval without excessive resource cost.

### Object detection

- modern small YOLO-class model or an equivalent compact detector;
- only load when spatial detection is required.

## Development roadmap

### Phase M0 — Model and resource management foundation

**Objective:** create the architecture required to safely host multiple local AI capabilities.

**Includes:** model inventory, provider registry, hardware detection, lifecycle state machine, CPU/GPU placement, VRAM/RAM budgets, load/unload, queues, cancellation, idle eviction and benchmark profiles.

**Priority:** Essential  
**Complexity:** High  
**Hardware impact:** Light when idle; controls all later heavy workloads.  
**Dependencies:** existing local runtime/provider architecture.  
**Stage:** MVP foundation for the multimodal roadmap.

No later multimodal phase should bypass this layer with independent ad-hoc model loading.

### Phase M1 — Semantic intelligence and retrieval

**Objective:** upgrade AIP memory and knowledge retrieval from basic stored context to semantic retrieval.

**Includes:**

- text embeddings;
- shared sentence similarity;
- local vector index;
- retrieval API;
- reranking;
- memory/RAG integration;
- provenance and bounded context assembly.

**Priority:** Essential  
**Complexity:** High  
**Hardware impact:** Light to Moderate depending on model placement.  
**Dependencies:** M0, cognitive/memory authority.  
**Stage:** MVP/post-MVP boundary, with memory retrieval prioritized before broader multimodality.

Candidate families: BGE-M3/E5/BGE multilingual + a small multilingual reranker.

### Phase M2 — First-party local voice stack

**Objective:** make voice a normal offline AIP capability rather than an external-provider-only workflow.

**Includes:**

- VAD;
- ASR;
- TTS;
- provider/model lifecycle integration;
- microphone/output selection;
- cancellation;
- explicit listening state;
- bounded audio memory;
- PT-BR validation;
- Windows first-party setup;
- later platform-appropriate Android runtime.

**Priority:** Essential  
**Complexity:** High  
**Hardware impact:** Moderate to Heavy during ASR/TTS execution.  
**Dependencies:** M0; existing Phase 8 voice safety/product UX.  
**Stage:** Post-MVP, high user value.

VAD should preferably remain CPU-resident. ASR/TTS residency must be benchmark-driven.

### Phase M3 — Local visual understanding

**Objective:** let the agent understand screenshots, interfaces, images and photographs locally.

**Includes:**

- compact VLM;
- VQA through the same VLM when adequate;
- explicit capture/image input;
- model lifecycle integration;
- contextual OCR/visual description;
- bounded privacy-aware visual context;
- screen-vision integration.

**Priority:** High  
**Complexity:** Very High  
**Hardware impact:** Heavy  
**Dependencies:** M0 and existing Phase 11 capture/privacy architecture.  
**Stage:** Post-MVP.

Candidate direction: quantized Qwen-VL/Qwen multimodal 3B–4B-class or a benchmarked equivalent.

### Phase M4 — Document intelligence

**Objective:** answer questions over local documents without requiring a dedicated Document-QA model by default.

**Includes:**

- parsers;
- chunking;
- metadata/provenance;
- embeddings;
- vector retrieval;
- reranking;
- context assembly;
- LLM answering;
- VLM fallback for visual/complex pages;
- PDF/document privacy boundaries.

**Priority:** High  
**Complexity:** High  
**Hardware impact:** Moderate; Heavy only when VLM fallback is needed.  
**Dependencies:** M1; M3 for visual-document fallback.  
**Stage:** Post-MVP.

A dedicated Document-QA model is deferred unless benchmarks demonstrate a clear need.

### Phase M5 — Visual memory

**Objective:** allow AIP to semantically index and retrieve user-approved visual memories.

**Includes:**

- image embeddings;
- visual index;
- similarity retrieval;
- provenance;
- explicit retention controls;
- privacy/storage policy;
- integration with agent memory without treating raw image bytes as automatically durable memory.

**Priority:** High  
**Complexity:** High  
**Hardware impact:** Moderate during embedding; otherwise light.  
**Dependencies:** M0, memory authority, ideally M3.  
**Stage:** Later post-MVP.

Candidate direction: SigLIP/CLIP-class encoder or a better benchmarked successor.

### Phase M6 — Spatial visual perception

**Objective:** give the agent deterministic object locations when VLM text alone is insufficient.

**Includes:**

- compact object detector;
- labels;
- bounding boxes;
- confidence;
- optional fusion with VLM context;
- on-demand model loading only.

**Priority:** Medium  
**Complexity:** High  
**Hardware impact:** Moderate while running.  
**Dependencies:** M0; M3 recommended.  
**Stage:** Long-term / advanced perception.

Candidate direction: small modern YOLO-class detector or equivalent.

## Recommended development order

1. M0 — model/resource manager;
2. M1 — embeddings, similarity, reranking and semantic memory/RAG;
3. M2 — VAD + ASR + TTS;
4. M3 — VLM + VQA;
5. M4 — document RAG with VLM fallback;
6. M5 — visual embeddings/memory;
7. M6 — object detection.

This order prioritizes infrastructure and capabilities that improve the existing agent before adding heavier perception stacks.

## Benchmark gates before model decisions

Do not permanently select candidate models from marketing/spec-sheet claims.

Each family must pass AIP-specific tests on the reference PC covering, where applicable:

- VRAM peak and steady state;
- RAM use;
- cold load time;
- warm load time;
- unload/reclaim behavior;
- tokens/s or real-time factor;
- first-result latency;
- CPU offload cost;
- quality in PT-BR;
- coding/reasoning/tool quality for the LLM;
- retrieval recall/precision for embeddings;
- reranking gain;
- ASR WER on PT-BR speech;
- TTS intelligibility/naturalness/latency;
- VLM screenshot/UI/OCR/VQA accuracy;
- image retrieval quality;
- object-detection accuracy and latency;
- behavior when another heavy model must be evicted.

Model choice remains provisional until these benchmarks exist.

## Main technical risks

- model churn making hard-coded provider choices obsolete;
- VRAM fragmentation or unreliable reclamation between heavy models;
- excessive model-switch latency damaging conversational UX;
- CPU offload making a theoretically fitting architecture too slow;
- duplicate capabilities creating unnecessary model/runtime complexity;
- VLM OCR/GUI hallucination;
- retrieval pollution from low-quality embeddings or memory ingestion;
- reranking latency for oversized candidate sets;
- audio/visual privacy regressions;
- hidden background listeners/capture;
- platform divergence between Windows and Android;
- provider process lifecycle failures;
- trying to keep too many models resident simultaneously.

## Architectural decisions already favored by this roadmap

- modular specialized models instead of a giant Any-to-Any model;
- LLM remains central orchestrator;
- embeddings and sentence similarity share a model by default;
- VQA shares the VLM by default;
- Document QA is a retrieval pipeline by default;
- VLM handles visually complex document pages when required;
- VAD is a strong CPU-resident candidate;
- object detection is on-demand;
- heavy models are not assumed to coexist in 8 GB VRAM;
- model/resource lifecycle is a prerequisite, not an afterthought.

## Decisions still requiring benchmark/prototype evidence

- final LLM family/quantization;
- final multilingual embedding model;
- whether reranker should remain CPU-resident;
- ASR model size that provides the best PT-BR quality/latency point;
- final TTS runtime/model and future voice-identity path;
- VLM family and quantization;
- whether the LLM must temporarily unload for VLM/ASR/TTS on the reference GPU;
- image embedding model;
- object detector;
- vector database/index implementation;
- exact warm-cache and eviction timings;
- Windows vs Android model/runtime differences.

Until those gates are passed, candidate model names are evaluation targets, not promises.
