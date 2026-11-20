import { Calculator, Clock3, Cpu, Wrench } from 'lucide-react'

export const prompts = [
  {
    name: 'calculator',
    label: 'Calculator',
    icon: Calculator,
    title: 'Run a calculation',
    prompt: 'What is 157 multiplied by 243?',
  },
  {
    name: 'get_current_datetime',
    label: 'Local time',
    icon: Clock3,
    title: 'Check local time',
    prompt: 'What is the current local date and time?',
  },
  {
    name: 'get_system_information',
    label: 'Device information',
    icon: Cpu,
    title: 'Inspect this device',
    prompt: 'What operating system is this device running?',
  },
]

export const toolName = (name: string) => prompts.find((p) => p.name === name)?.label || name
export const toolIcon = (name: string) => prompts.find((p) => p.name === name)?.icon || Wrench
