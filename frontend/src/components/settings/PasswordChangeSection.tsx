import { memo, useState, useCallback, useMemo } from "react";
import { KeyRound } from "lucide-react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import ErrorMessage from "../common/feedback/ErrorMessage";
import SettingsCard from "./SettingsCard";
import {
  settingsErrorClass,
  settingsHelperClass,
  settingsInputClass,
  settingsLabelClass,
  settingsPrimaryButtonClass,
} from "./settingsUi";
import { authService } from "../../services/auth";
import { getErrorMessage } from "../../utils/error";

interface PasswordFormValues {
  current_password: string;
  new_password: string;
  confirm_password: string;
}

const PasswordChangeSection = memo(function PasswordChangeSection() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const passwordSchema = useMemo(() => {
    return z
      .object({
        current_password: z.string().min(1, "Current password is required"),
        new_password: z
          .string()
          .min(8, "New password must be between 8 and 64 characters")
          .max(64, "New password must be between 8 and 64 characters")
          .refine((v) => /[A-Za-z]/.test(v) && /[0-9]/.test(v), {
            message:
              "New password must contain at least one letter and one digit",
          }),
        confirm_password: z.string(),
      })
      .superRefine((data, ctx) => {
        if (data.new_password !== data.confirm_password) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: "New password and confirmation do not match",
            path: ["confirm_password"],
          });
        }
      });
  }, []);

  const {
    register,
    handleSubmit,
    reset,
    setError: setFormError,
    formState: { errors },
  } = useForm<PasswordFormValues>({
    resolver: zodResolver(passwordSchema),
    defaultValues: {
      current_password: "",
      new_password: "",
      confirm_password: "",
    },
    mode: "onBlur",
  });

  const onSubmit = useCallback(async (data: PasswordFormValues) => {
    setError(null);
    setSuccess(null);
    setLoading(true);
    try {
      await authService.changePassword({
        current_password: data.current_password,
        new_password: data.new_password,
      });
      setSuccess("Password changed");
      reset();
    } catch (err) {
      const message = getErrorMessage(err, "Failed to change password");
      if (
        message.includes("between 8 and 64") ||
        message.includes("one letter and one digit") ||
        message.includes("New password must")
      ) {
        setFormError("new_password", { message });
      } else if (
        message.toLowerCase().includes("password") ||
        message.includes("密码")
      ) {
        setFormError("current_password", { message });
      } else {
        setError(message);
      }
    } finally {
      setLoading(false);
    }
  }, [reset, setFormError]);

  const handleCloseError = useCallback(() => setError(null), []);
  const handleCloseSuccess = useCallback(() => setSuccess(null), []);

  return (
    <SettingsCard
      id="security"
      title="Security"
      description="Change your password periodically and avoid reusing it across services."
      icon={
        <KeyRound className="h-[clamp(1rem,2.25vw,1.25rem)] w-[clamp(1rem,2.25vw,1.25rem)]" aria-hidden="true" data-oid="8hmxl4n" />
      }
      data-oid="ne71trr"
    >
      {error && (
        <ErrorMessage
          message={error}
          onClose={handleCloseError}
          type="error"
          autoDismissMs={5000}
          className="mb-[clamp(0.78rem,1.8vw,1rem)] [&_p]:text-[length:var(--settings-text-sm)]"
          data-oid="h9n6kem"
        />
      )}
      {success && (
        <ErrorMessage
          message={success}
          onClose={handleCloseSuccess}
          type="info"
          autoDismissMs={3000}
          className="mb-[clamp(0.78rem,1.8vw,1rem)] [&_p]:text-[length:var(--settings-text-sm)]"
          data-oid="s.:7sdp"
        />
      )}
      <form
        onSubmit={handleSubmit(onSubmit)}
        noValidate
        className="grid gap-[clamp(0.78rem,1.8vw,1rem)]"
        data-oid="be5kts_"
      >
        <div data-oid="0h03yxf">
          <label
            htmlFor="current-password"
            className={settingsLabelClass()}
            data-oid="6.-il:2"
          >
            Current password
          </label>
          <input
            id="current-password"
            type="password"
            {...register("current_password")}
            required
            aria-invalid={Boolean(errors.current_password)}
            aria-describedby={
              errors.current_password
                ? "current-password-error"
                : undefined
            }
            className={settingsInputClass(Boolean(errors.current_password))}
            data-oid="8_-nvyf"
          />

          {errors.current_password && (
            <p
              id="current-password-error"
              className={settingsErrorClass()}
              data-oid="hvyxc0c"
            >
              {errors.current_password.message}
            </p>
          )}
        </div>
        <div data-oid="hp88ntt">
          <label
            htmlFor="new-password"
            className={settingsLabelClass()}
            data-oid="ig:8b27"
          >
            New password
          </label>
          <input
            id="new-password"
            type="password"
            {...register("new_password")}
            required
            minLength={8}
            maxLength={64}
            aria-invalid={Boolean(errors.new_password)}
            aria-describedby={
              errors.new_password ? "new-password-error" : undefined
            }
            className={settingsInputClass(Boolean(errors.new_password))}
            data-oid="bnlj:z1"
          />

          {errors.new_password ? (
            <p
              id="new-password-error"
              className={settingsErrorClass()}
              data-oid="6-qjqxf"
            >
              {errors.new_password.message}
            </p>
          ) : (
            <p
              className={settingsHelperClass()}
              data-oid=".--qsoi"
            >
              8–64 characters, include at least one letter and one digit
            </p>
          )}
        </div>
        <div data-oid="47ix9rd">
          <label
            htmlFor="confirm-password"
            className={settingsLabelClass()}
            data-oid="5xmgdgc"
          >
            Confirm new password
          </label>
          <input
            id="confirm-password"
            type="password"
            {...register("confirm_password")}
            required
            minLength={8}
            maxLength={64}
            aria-invalid={Boolean(errors.confirm_password)}
            aria-describedby={
              errors.confirm_password
                ? "confirm-password-error"
                : undefined
            }
            className={settingsInputClass(Boolean(errors.confirm_password))}
            data-oid=".8op07u"
          />

          {errors.confirm_password && (
            <p
              id="confirm-password-error"
              className={settingsErrorClass()}
              data-oid="3:cc56f"
            >
              {errors.confirm_password.message}
            </p>
          )}
        </div>
        <div
          data-testid="settings-security-actions"
          className="mt-auto flex justify-end border-t border-[var(--settings-panel-border)] pt-[clamp(0.78rem,1.8vw,1rem)]"
        >
          <button
            type="submit"
            disabled={loading}
            className={settingsPrimaryButtonClass("w-full sm:w-auto sm:min-w-[10rem]")}
            data-oid="-zsxz4-"
          >
            {loading ? "Changing..." : "Change password"}
          </button>
        </div>
      </form>
    </SettingsCard>
  );
});

export default PasswordChangeSection;
