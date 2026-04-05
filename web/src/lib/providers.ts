export interface ProviderMeta {
  id: string;
  name: string;
  company: string;
  color: string;
  defaultModels: string[];
  defaultEndpoint: string;
  description: string;
  authHint: string;
}

export const PROVIDERS: ProviderMeta[] = [
  {
    id: 'claude',
    name: 'Claude',
    company: 'Anthropic',
    color: '#D97757',
    defaultModels: ['claude-sonnet-4-20250514', 'claude-haiku-4-5-20251001', 'claude-opus-4-20250514'],
    defaultEndpoint: 'https://api.anthropic.com/v1/messages',
    description: "Anthropic's Claude models for advanced reasoning",
    authHint: 'Get your API key from console.anthropic.com',
  },
  {
    id: 'openai',
    name: 'GPT',
    company: 'OpenAI',
    color: '#10A37F',
    defaultModels: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'o3-mini'],
    defaultEndpoint: 'https://api.openai.com/v1/chat/completions',
    description: 'OpenAI GPT models',
    authHint: 'Get your API key from platform.openai.com',
  },
  {
    id: 'gemini',
    name: 'Gemini',
    company: 'Google',
    color: '#4285F4',
    defaultModels: ['gemini-2.5-pro', 'gemini-2.5-flash', 'gemini-2.0-flash'],
    defaultEndpoint: 'https://generativelanguage.googleapis.com/v1beta',
    description: "Google's Gemini multimodal models",
    authHint: 'Get your API key from aistudio.google.com',
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    company: 'DeepSeek',
    color: '#4D6BFE',
    defaultModels: ['deepseek-chat', 'deepseek-reasoner'],
    defaultEndpoint: 'https://api.deepseek.com/v1/chat/completions',
    description: 'DeepSeek models (OpenAI-compatible)',
    authHint: 'Get your API key from platform.deepseek.com',
  },
  {
    id: 'copilot',
    name: 'Copilot',
    company: 'GitHub',
    color: '#8957E5',
    defaultModels: ['gpt-4o', 'Mistral-large', 'Meta-Llama-3.1-405B-Instruct'],
    defaultEndpoint: 'https://models.inference.ai.azure.com/chat/completions',
    description: 'GitHub Models via Copilot API',
    authHint: 'Use a GitHub Personal Access Token',
  },
  {
    id: 'minimax',
    name: 'MiniMax',
    company: 'MiniMax',
    color: '#FF6B35',
    defaultModels: ['MiniMax-Text-01', 'abab6.5s-chat'],
    defaultEndpoint: 'https://api.minimax.chat/v1/text/chatcompletion_v2',
    description: 'MiniMax large language models',
    authHint: 'Get your API key from api.minimax.chat',
  },
  {
    id: 'glm',
    name: 'GLM',
    company: 'Zhipu AI',
    color: '#00D4AA',
    defaultModels: ['glm-4-plus', 'glm-4-flash', 'glm-4'],
    defaultEndpoint: 'https://open.bigmodel.cn/api/paas/v4/chat/completions',
    description: 'Zhipu AI GLM models',
    authHint: 'Get your API key from open.bigmodel.cn',
  },
];
