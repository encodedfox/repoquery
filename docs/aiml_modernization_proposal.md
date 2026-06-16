# AI/ML Web References Modernization Proposal

**Status**: Proposal for Task 13  
**Date**: 2025-12-10  
**Current State**: 5 basic AI/ML references from 2023 (stale)  
**Proposed**: Comprehensive modern structure with 50+ repos

## Current AI/ML Section (Outdated)

From `web_references.yml` - 5 references, all from 2023:
1. Data Pitfalls - Startup Edition
2. How to Become a Data Scientist
3. How NOT to Hire Your First Data Scientist
4. AI Hierarchy of Needs
5. Blackboard System

**Issues**: 
- No coverage of LLMs, Vector DBs, RAG, Fine-tuning, Agents
- Content is 2 years old (pre-LLM era)
- Missing modern frameworks and tools

## Proposed Modern Structure

### 1. Foundation Models & LLMs

**High-Priority Repos** (from starred list):
- `deepseek-ai/DeepSeek-V3` ⭐️100,652 - Latest LLM
- `deepseek-ai/DeepSeek-Coder` ⭐️22,471 - Code LLM
- `deepseek-ai/DeepSeek-VL2` ⭐️5,146 - Vision-language
- `deepseek-ai/DeepSeek-R1` ⭐️91,566 - Reasoning model
- `deepseek-ai/Janus` ⭐️17,637 - Multimodal
- `deepseek-ai/ESFT` ⭐️712 - Fine-tuning
- `Jiayi-Pan/TinyZero` ⭐️12,469 - R1-Zero reproduction

**Recommended Web References**:
- DeepSeek official blog/papers
- Transformer architecture explainers
- LLM evaluation frameworks
- Model cards and documentation

### 2. Vector Databases & Search

**High-Priority Repos**:
- `chroma-core/chroma` ⭐️24,826 - Vector database for AI
- `weaviate/weaviate` ⭐️15,185 - Vector database with filtering

**Recommended Web References**:
- Vector database comparison guides
- Embedding model selection
- Similarity search algorithms
- Index optimization techniques

### 3. RAG & Context Management

**High-Priority Repos**:
- `HKUDS/LightRAG` ⭐️25,654 - RAG implementation
- `mem0ai/mem0` ⭐️44,063 - Universal memory layer for AI agents
- `GreatScottyMac/context-portal` ⭐️702 - Knowledge graph for RAG

**Recommended Web References**:
- RAG architecture patterns
- Context window optimization
- Retrieval strategies
- Knowledge graph construction

### 4. Model Evaluation & Benchmarking

**High-Priority Repos**:
- `stanford-crfm/helm` ⭐️2,571 - Holistic evaluation framework
- `SCLBD/DeepfakeBench` ⭐️928 - Deepfake detection benchmark
- `amazon-science/RefChecker` ⭐️405 - Hallucination detection
- `aws/fmeval` ⭐️272 - Foundation model evaluation

**Recommended Web References**:
- LLM benchmarking best practices
- Evaluation metrics for different tasks
- Bias and fairness testing
- Model comparison frameworks

### 5. Training & Fine-tuning

**High-Priority Repos**:
- `aws-neuron/aws-neuron-sdk` ⭐️557 - AWS ML chips
- `aws-neuron/neuronx-distributed` ⭐️63 - Distributed training
- `skypilot-org/skypilot` ⭐️9,070 - Multi-cloud ML workloads
- `aws-samples/amazon-bedrock-workshop` ⭐️2,044 - Bedrock workshop
- `aws-samples/amazon-nova-samples` ⭐️389 - Nova samples

**Recommended Web References**:
- LoRA and QLoRA techniques
- PEFT (Parameter-Efficient Fine-Tuning)
- Distributed training strategies
- GPU optimization guides

### 6. Agents & Orchestration

**High-Priority Repos**:
- `awslabs/mcp` ⭐️7,595 - AWS MCP servers
- `langchain-ai/executive-ai-assistant` ⭐️2,136 - AI assistant
- `The-Swarm-Corporation/Zero` ⭐️13 - Workflow automation
- `dnhkng/GLaDOS` ⭐️5,157 - AI personality core

**Recommended Web References**:
- Multi-agent systems design
- LangChain/LlamaIndex guides
- Prompt engineering techniques
- Agent orchestration patterns

### 7. ML Operations & Infrastructure

**High-Priority Repos**:
- `PrefectHQ/prefect` ⭐️21,050 - Workflow orchestration
- `neuml/txtai` ⭐️11,900 - AI framework
- `aws-samples/aws-etl-orchestrator` ⭐️343 - ETL orchestration
- `kestra-io/kestra` ⭐️25,932 - Workflow orchestration

**Recommended Web References**:
- MLOps best practices
- Model deployment strategies
- Monitoring and observability
- Cost optimization

### 8. Generative AI Tools & Frameworks

**High-Priority Repos**:
- `FissionAI/FloTorch` ⭐️265 - GenAI optimization on AWS
- `salesforce/progen` ⭐️683 - ProGen models
- `AntonOsika/gpt-engineer` ⭐️55,100 - Code generation
- `Doriandarko/claude-engineer` ⭐️11,136 - Claude-based engineer

**Recommended Web References**:
- Prompt engineering guides
- Code generation best practices
- GenAI application patterns
- Safety and alignment

### 9. Data & Preprocessing

**High-Priority Repos**:
- `allenai/dolma` ⭐️1,356 - Data processing for LLMs
- `amazon-science/chronos-forecasting` ⭐️4,474 - Time series
- `awslabs/python-deequ` ⭐️806 - Data quality
- `awslabs/gluonts` ⭐️5,085 - Time series modeling

**Recommended Web References**:
- Data preparation for ML
- Tokenization strategies
- Data quality for LLMs
- Synthetic data generation

### 10. AWS-Specific AI/ML

**High-Priority Repos**:
- `aws-samples/genai-quickstart-pocs` ⭐️393
- `aws-samples/amazon-bedrock-samples` ⭐️1,314
- `aws-samples/data-insights-with-amazon-q-business` ⭐️18
- `aws-samples/aws-genai-llm-chatbot` ⭐️1,351

**Recommended Web References**:
- Amazon Bedrock documentation
- SageMaker best practices
- AWS Neuron optimization
- Cost-effective ML on AWS

## Implementation Plan

### Phase 1: Add New Categories (Priority)

Update `web_references.yml` with new sections:

```yaml
# Foundation Models & LLMs
- id: "deepseek-docs"
  title: "DeepSeek Model Documentation"
  url: "https://github.com/deepseek-ai/DeepSeek-V3"
  category: "AI/ML"
  subcategory: "Foundation Models"
  related_repos:
    - "deepseek-ai/DeepSeek-V3"
    - "deepseek-ai/DeepSeek-Coder"

# Vector Databases
- id: "chroma-docs"
  title: "Chroma Vector Database"
  url: "https://docs.trychroma.com/"
  category: "AI/ML"
  subcategory: "Vector Databases"
  related_repos:
    - "chroma-core/chroma"
    - "weaviate/weaviate"

# Continue for all categories...
```

### Phase 2: Update Existing References

Keep historical references but mark as foundational:
- Update last_verified dates
- Add difficulty levels
- Link to modern implementations

### Phase 3: Organize by Learning Path

1. **Fundamentals** (Intro)
   - Data hierarchy of needs (existing)
   - ML basics

2. **Infrastructure** (Intermediate)
   - Vector databases
   - Model serving
   - GPU computing

3. **Techniques** (Advanced)
   - RAG patterns
   - Fine-tuning methods
   - Prompt engineering

4. **Applications** (Practical)
   - Agent systems
   - Code generation
   - Evaluation

## Statistics

**Current**: 5 AI/ML references (2023)  
**Proposed**: 50+ references (2024-2025)  
**Starred Repos**: 50+ AI/ML related  
**Priority**: HIGH (2-year content gap)

## Next Steps

1. Review this proposal
2. Update `data/canonical/web_references.yml`
3. Add new references with modern content
4. Link to relevant starred repos
5. Regenerate documentation

---

*This proposal identifies gaps and provides structure for comprehensive AI/ML coverage based on the 50+ AI/ML repos in the starred collection.*