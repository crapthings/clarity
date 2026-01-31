import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export const useAppStore = create(
  persist(
    (set) => ({
      currentPage: 'trace', // trace, summary, settings
      setCurrentPage: (page) => set({ currentPage: page }),
      language: 'en', // en, zh
      setLanguage: (lang) => set({ language: lang })
    }),
    {
      name: 'clarity-storage' // localStorage key
    }
  )
)
