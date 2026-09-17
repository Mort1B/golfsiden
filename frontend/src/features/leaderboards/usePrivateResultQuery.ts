import { useQuery, useQueryClient, type QueryFunctionContext, type QueryKey } from '@tanstack/react-query'
import { isPrivateResultDenial, loadPrivateResult, type PrivateResultScope } from '../../api/privateResults'
interface Options<T> {
  queryKey: QueryKey
  queryFn: (context: QueryFunctionContext) => Promise<T>
  enabled?: boolean
  retry?: false
}
export function usePrivateResultQuery<T>(scope: PrivateResultScope, options: Options<T>) {
  const client = useQueryClient()
  return useQuery({ ...options,
    queryFn: context => loadPrivateResult(client, scope, options.queryKey, context.signal, () => options.queryFn(context)),
    retry: (count, error) => options.retry !== false && !isPrivateResultDenial(error) && count < 1,
  })
}
