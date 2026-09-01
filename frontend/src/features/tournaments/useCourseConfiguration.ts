import { useRef } from 'react'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { CourseSelection, ProviderTee } from '../../api/courses'
import { courseApi, courseKeys } from '../../api/courses'
import { tournamentKeys } from '../../api/tournaments'
import type { Round } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { configurationFailure, validateCatalogSearch } from './courseConfiguration'
import type { ConfigurationFailure } from './courseConfiguration'

interface Input {
  tournamentId: string
  round: Round
  providerCourseId: string
  catalogQuery: string
  expanded: boolean
}

type SaveResult =
  | { configured: Round; failure: null }
  | { configured: null; failure: ConfigurationFailure | null }

export function useCourseConfiguration({ tournamentId, round, providerCourseId, catalogQuery, expanded }: Input) {
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const queryClient = useQueryClient()
  const submitting = useRef(false)
  const catalogSearch = validateCatalogSearch(catalogQuery)
  const catalog = useQuery({
    queryKey: courseKeys.catalog(userId, tournamentId, catalogSearch.normalized),
    queryFn: () => courseApi.catalog(tournamentId, catalogSearch.normalized),
    enabled: expanded && catalogSearch.ok && userId.length > 0 && round.status === 'draft',
    placeholderData: keepPreviousData,
  })
  const detail = useQuery({
    queryKey: courseKeys.provider(userId, tournamentId, providerCourseId),
    queryFn: () => courseApi.provider(tournamentId, providerCourseId),
    enabled: expanded && userId.length > 0 && round.status === 'draft' && providerCourseId.length > 0,
    placeholderData: keepPreviousData,
  })
  const mutation = useMutation({
    mutationFn: (selection: CourseSelection) => {
      const csrfToken = auth.session?.csrf_token
      if (!csrfToken) throw new Error('Økten mangler. Logg inn på nytt.')
      return courseApi.configure(round.id, tournamentId, round.updated_at, selection, csrfToken)
    },
  })

  const save = async (selection: CourseSelection): Promise<SaveResult> => {
    if (submitting.current || round.status !== 'draft') return { configured: null, failure: null }
    submitting.current = true
    mutation.reset()
    try {
      const configured = await mutation.mutateAsync(selection)
      queryClient.setQueryData(tournamentKeys.round(userId, round.id), configured)
      queryClient.setQueryData<Round[]>(tournamentKeys.rounds(userId, tournamentId), (current) =>
        current?.map((item) => item.id === configured.id ? configured : item))
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: tournamentKeys.round(userId, round.id), exact: true }),
        queryClient.invalidateQueries({ queryKey: tournamentKeys.rounds(userId, tournamentId), exact: true }),
      ])
      return { configured, failure: null }
    } catch (error) {
      const failure = configurationFailure(error instanceof Error ? error : new Error('Ukjent feil'))
      if (failure === 'stale' || failure === 'not-draft') {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: tournamentKeys.round(userId, round.id), exact: true }),
          queryClient.invalidateQueries({ queryKey: tournamentKeys.rounds(userId, tournamentId), exact: true }),
        ])
      }
      if (failure === 'tee-stale' && providerCourseId) {
        await queryClient.invalidateQueries({
          queryKey: courseKeys.provider(userId, tournamentId, providerCourseId), exact: true,
        })
      }
      return { configured: null, failure }
    } finally {
      submitting.current = false
    }
  }

  const detailIsCurrent = detail.data?.provider_course_id === providerCourseId
  return {
    catalog,
    catalogSearch,
    catalogIsCurrent: catalogSearch.ok && !catalog.isPlaceholderData && !catalog.error,
    detail,
    detailIsCurrent,
    mutation,
    save,
    saveProvider: (tee: ProviderTee) => save({
      source: 'golf_course_api', provider_course_id: providerCourseId,
      tee: { category: tee.category, name: tee.name },
    }),
  }
}
