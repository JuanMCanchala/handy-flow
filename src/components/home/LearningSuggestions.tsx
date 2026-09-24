import React, { useCallback, useEffect, useState } from "react";
import { BookPlus, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands, type LearningCandidate } from "@/bindings";
import { Button } from "../ui/Button";

/**
 * Self-learning dictionary suggestions banner: shown on Home when a
 * candidate replacement (harvested from diffing user edits, see
 * `src-tauri/src/learning.rs`) has been seen more than twice. Each
 * suggestion can be one-click added to the custom words dictionary or
 * dismissed.
 */
export const LearningSuggestions: React.FC = () => {
  const { t } = useTranslation();
  const [candidates, setCandidates] = useState<LearningCandidate[]>([]);

  const load = useCallback(async () => {
    const result = await commands.getLearningCandidates();
    if (result.status === "ok") {
      setCandidates(result.data);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleAdd = async (candidate: LearningCandidate) => {
    setCandidates((prev) => prev.filter((c) => c.id !== candidate.id));
    const result = await commands.addLearningCandidateToDictionary(
      candidate.id,
      candidate.phrase_to,
    );
    if (result.status !== "ok") {
      toast.error(t("settings.home.learning.addError"));
      load();
    }
  };

  const handleDismiss = async (candidate: LearningCandidate) => {
    setCandidates((prev) => prev.filter((c) => c.id !== candidate.id));
    const result = await commands.dismissLearningCandidate(candidate.id);
    if (result.status !== "ok") {
      load();
    }
  };

  if (candidates.length === 0) {
    return null;
  }

  return (
    <div className="bg-surface border border-border rounded-lg divide-y divide-border">
      {candidates.map((candidate) => (
        <div
          key={candidate.id}
          className="px-4 py-3 flex items-center justify-between gap-3"
        >
          <p className="text-body text-text">
            {t("settings.home.learning.suggestion", {
              from: candidate.phrase_from,
              to: candidate.phrase_to,
            })}
          </p>
          <div className="flex items-center gap-2 shrink-0">
            <Button
              onClick={() => handleAdd(candidate)}
              variant="secondary"
              size="sm"
            >
              <BookPlus width={14} height={14} />
              <span>{t("settings.home.learning.addToDictionary")}</span>
            </Button>
            <Button
              onClick={() => handleDismiss(candidate)}
              variant="ghost"
              size="sm"
              title={t("settings.home.learning.dismiss")}
            >
              <X width={14} height={14} />
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
};
