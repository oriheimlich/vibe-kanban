import { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import RepoBranchSelector from '@/components/tasks/RepoBranchSelector';
import { ExecutorProfileSelector } from '@/components/settings';
import { useScheduleExecution } from '@/hooks/useScheduledExecution';
import {
  useTask,
  useRepoBranchSelection,
  useProjectRepos,
} from '@/hooks';
import { useProject } from '@/contexts/ProjectContext';
import { useUserSystem } from '@/components/ConfigProvider';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/lib/modals';
import type { ExecutorProfileId, JsonValue } from 'shared/types';
import { useKeySubmitTask, Scope } from '@/keyboard';

export interface ScheduleExecutionDialogProps {
  taskId: string;
}

const ScheduleExecutionDialogImpl =
  NiceModal.create<ScheduleExecutionDialogProps>(({ taskId }) => {
    const modal = useModal();
    const { projectId } = useProject();
    const { t } = useTranslation('tasks');
    const { profiles, config } = useUserSystem();
    const { schedule, isScheduling, error } = useScheduleExecution({
      onSuccess: () => {
        modal.hide();
      },
    });

    const [userSelectedProfile, setUserSelectedProfile] =
      useState<ExecutorProfileId | null>(null);
    const [scheduledAt, setScheduledAt] = useState('');

    const { isLoading: isLoadingTask } = useTask(taskId, {
      enabled: modal.visible,
    });

    const { data: projectRepos = [], isLoading: isLoadingRepos } =
      useProjectRepos(projectId, { enabled: modal.visible });

    const {
      configs: repoBranchConfigs,
      isLoading: isLoadingBranches,
      setRepoBranch,
      getWorkspaceRepoInputs,
      reset: resetBranchSelection,
    } = useRepoBranchSelection({
      repos: projectRepos,
      enabled: modal.visible && projectRepos.length > 0,
    });

    useEffect(() => {
      if (!modal.visible) {
        setUserSelectedProfile(null);
        setScheduledAt('');
        resetBranchSelection();
      }
    }, [modal.visible, resetBranchSelection]);

    const defaultProfile: ExecutorProfileId | null = useMemo(
      () => config?.executor_profile ?? null,
      [config?.executor_profile]
    );

    const effectiveProfile = userSelectedProfile ?? defaultProfile;

    const isLoadingInitial =
      isLoadingRepos || isLoadingBranches || isLoadingTask;

    const allBranchesSelected = repoBranchConfigs.every(
      (c) => c.targetBranch !== null
    );

    const scheduledAtDate = scheduledAt ? new Date(scheduledAt) : null;
    const isScheduledInFuture =
      scheduledAtDate !== null && scheduledAtDate > new Date();

    const canSchedule = Boolean(
      effectiveProfile &&
        allBranchesSelected &&
        projectRepos.length > 0 &&
        isScheduledInFuture &&
        !isScheduling &&
        !isLoadingInitial
    );

    const handleSchedule = async () => {
      if (
        !effectiveProfile ||
        !allBranchesSelected ||
        !projectId ||
        !scheduledAtDate ||
        projectRepos.length === 0
      )
        return;

      try {
        const repos = getWorkspaceRepoInputs();

        await schedule({
          taskId,
          projectId,
          scheduledAt: scheduledAtDate.toISOString(),
          executorProfileId: effectiveProfile as unknown as JsonValue,
          repos: repos.map((r) => ({
            repoId: r.repo_id,
            targetBranch: r.target_branch,
          })),
        });
      } catch (err) {
        console.error('Failed to schedule execution:', err);
      }
    };

    const handleOpenChange = (open: boolean) => {
      if (!open) modal.hide();
    };

    useKeySubmitTask(handleSchedule, {
      enabled: modal.visible && canSchedule,
      scope: Scope.DIALOG,
      preventDefault: true,
    });

    // Compute min datetime for the input (current time, rounded to minutes)
    const minDateTime = useMemo(() => {
      const now = new Date();
      now.setSeconds(0, 0);
      return now.toISOString().slice(0, 16);
    }, []);

    return (
      <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>
              {t('scheduleExecutionDialog.title')}
            </DialogTitle>
            <DialogDescription>
              {t('scheduleExecutionDialog.description')}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <label
                htmlFor="scheduled-at"
                className="text-sm font-medium leading-none"
              >
                {t('scheduleExecutionDialog.scheduledAt')}
              </label>
              <input
                id="scheduled-at"
                type="datetime-local"
                min={minDateTime}
                value={scheduledAt}
                onChange={(e) => setScheduledAt(e.target.value)}
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>

            {profiles && (
              <div className="space-y-2">
                <ExecutorProfileSelector
                  profiles={profiles}
                  selectedProfile={effectiveProfile}
                  onProfileSelect={setUserSelectedProfile}
                  showLabel={true}
                />
              </div>
            )}

            <RepoBranchSelector
              configs={repoBranchConfigs}
              onBranchChange={setRepoBranch}
              isLoading={isLoadingBranches}
              className="space-y-2"
            />

            {error && (
              <div className="text-sm text-destructive">
                {t('scheduleExecutionDialog.error')}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => modal.hide()}
              disabled={isScheduling}
            >
              {t('common:buttons.cancel')}
            </Button>
            <Button onClick={handleSchedule} disabled={!canSchedule}>
              {isScheduling
                ? t('scheduleExecutionDialog.scheduling')
                : t('scheduleExecutionDialog.schedule')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  });

export const ScheduleExecutionDialog = defineModal<
  ScheduleExecutionDialogProps,
  void
>(ScheduleExecutionDialogImpl);
