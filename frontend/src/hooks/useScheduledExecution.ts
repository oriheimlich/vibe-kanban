import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { scheduledExecutionsApi } from '@/lib/api';
import type {
  CreateScheduledExecutionRequest,
  ScheduledExecution,
} from 'shared/types';

export const scheduledExecutionKeys = {
  all: ['scheduledExecutions'] as const,
  list: (projectId: string) =>
    [...scheduledExecutionKeys.all, 'list', projectId] as const,
};

export function useScheduledExecutions(
  projectId: string | undefined,
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: scheduledExecutionKeys.list(projectId ?? ''),
    queryFn: () => scheduledExecutionsApi.list(projectId!),
    enabled: !!projectId && (options?.enabled ?? true),
  });
}

type UseScheduleExecutionArgs = {
  onSuccess?: (scheduled: ScheduledExecution) => void;
};

export function useScheduleExecution({
  onSuccess,
}: UseScheduleExecutionArgs = {}) {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (data: CreateScheduledExecutionRequest) =>
      scheduledExecutionsApi.create(data),
    onSuccess: (newScheduled: ScheduledExecution) => {
      queryClient.invalidateQueries({
        queryKey: scheduledExecutionKeys.all,
      });
      onSuccess?.(newScheduled);
    },
  });

  return {
    schedule: mutation.mutateAsync,
    isScheduling: mutation.isPending,
    error: mutation.error,
  };
}

export function useCancelScheduledExecution() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (id: string) => scheduledExecutionsApi.cancel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: scheduledExecutionKeys.all,
      });
    },
  });

  return {
    cancel: mutation.mutateAsync,
    isCancelling: mutation.isPending,
    error: mutation.error,
  };
}
