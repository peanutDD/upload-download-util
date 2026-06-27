import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import type { User } from "../../types/auth";
import type { ApiToken } from "../../services/apiTokens";
import Settings from "../../pages/Settings";
import ThemeSection from "./ThemeSection";
import UserInfoSection from "./UserInfoSection";
import ApiTokenSection from "./ApiTokenSection";
import { settingsInputClass } from "./settingsUi";
import { useThemeStore } from "../../store/themeStore";
import { useAuthStore } from "../../store/authStore";
import { authService } from "../../services/auth";
import {
  useApiTokens,
  useCreateApiToken,
  useDeleteApiToken,
} from "../../hooks/useApiTokens";
import { useStorageUsage } from "../../hooks/useStorageUsage";
import { useOcrStatus } from "../../hooks/useOcrStatus";

vi.mock("../../store/themeStore", () => ({
  useThemeStore: vi.fn(),
}));

vi.mock("../../store/authStore", () => ({
  useAuthStore: vi.fn(),
}));

vi.mock("../../services/auth", () => ({
  authService: {
    sendEmailVerification: vi.fn(),
    checkProfileAvailability: vi.fn(),
    updateProfile: vi.fn(),
    changePassword: vi.fn(),
  },
}));

vi.mock("../../hooks/useApiTokens", () => ({
  useApiTokens: vi.fn(),
  useCreateApiToken: vi.fn(),
  useDeleteApiToken: vi.fn(),
}));

vi.mock("../../hooks/useStorageUsage", () => ({
  useStorageUsage: vi.fn(),
}));

vi.mock("../../hooks/useOcrStatus", () => ({
  useOcrStatus: vi.fn(),
}));

vi.mock("../../hooks/useClipboard", () => ({
  useClipboard: () => ({ copy: vi.fn().mockResolvedValue(true) }),
}));

const mockSetTheme = vi.fn();
const mockUpdateUser = vi.fn();
const mockCreateMutate = vi.fn();
const mockDeleteMutate = vi.fn();

const currentUser: User = {
  id: "user-1",
  username: "alice",
  email: "alice@example.com",
  created_at: "2026-05-04T00:00:00Z",
};

function asApiTokensQuery(data: ApiToken[], isLoading = false) {
  return { data, isLoading } as unknown as ReturnType<typeof useApiTokens>;
}

function asCreateTokenMutation() {
  return {
    isPending: false,
    mutate: mockCreateMutate,
  } as unknown as ReturnType<typeof useCreateApiToken>;
}

function asDeleteTokenMutation() {
  return {
    isPending: false,
    mutate: mockDeleteMutate,
  } as unknown as ReturnType<typeof useDeleteApiToken>;
}

function mockAuthStore() {
  vi.mocked(useAuthStore).mockImplementation((selector) =>
    selector({
      user: currentUser,
      token: "test-token",
      setAuth: vi.fn(),
      updateUser: mockUpdateUser,
      clearAuth: vi.fn(),
      isAuthenticated: () => true,
    }),
  );
}

function LocationProbe() {
  const location = useLocation();
  return (
    <div data-testid="location">
      {location.pathname}
      {location.search}
    </div>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(useThemeStore).mockReturnValue({
    theme: "dark",
    effectiveTheme: "dark",
    setTheme: mockSetTheme,
    toggleTheme: vi.fn(),
  });
  mockAuthStore();
  vi.mocked(authService.sendEmailVerification).mockResolvedValue({
    message: "Verification sent",
  });
  vi.mocked(authService.checkProfileAvailability).mockResolvedValue({
    username_available: true,
    email_available: true,
  });
  vi.mocked(authService.updateProfile).mockResolvedValue({
    user: { ...currentUser, email: "alice+new@example.com" },
  });
  vi.mocked(useApiTokens).mockReturnValue(asApiTokensQuery([]));
  vi.mocked(useCreateApiToken).mockReturnValue(asCreateTokenMutation());
  vi.mocked(useDeleteApiToken).mockReturnValue(asDeleteTokenMutation());
  vi.mocked(useStorageUsage).mockReturnValue({
    data: { total_size: 2048, file_count: 3 },
  } as unknown as ReturnType<typeof useStorageUsage>);
  vi.mocked(useOcrStatus).mockReturnValue({
    data: {
      enabled: false,
      pdf_max_pages: 5,
      tesseract: { bin: "tesseract", available: false },
      poppler: { bin: "pdftoppm", available: false },
    },
    isLoading: false,
  } as unknown as ReturnType<typeof useOcrStatus>);
});

describe("Settings page regressions", () => {
  it("keeps theme selection wired to the theme store", async () => {
    render(<ThemeSection />);

    await userEvent.click(screen.getByRole("button", { name: /Light/i }));

    expect(mockSetTheme).toHaveBeenCalledWith("light");
  });

  it("offers Terminal as the fourth theme option", async () => {
    render(<ThemeSection />);

    expect(screen.getByRole("button", { name: /Terminal/i })).toHaveTextContent(
      /neon terminal/i,
    );

    await userEvent.click(screen.getByRole("button", { name: /Terminal/i }));

    expect(mockSetTheme).toHaveBeenCalledWith("terminal");
  });

  it("offers Portfolio as the fifth theme option", async () => {
    render(<ThemeSection />);

    expect(screen.getByRole("button", { name: /Portfolio/i })).toHaveTextContent(
      /neon developer portfolio/i,
    );

    await userEvent.click(screen.getByRole("button", { name: /Portfolio/i }));

    expect(mockSetTheme).toHaveBeenCalledWith("portfolio");
  });

  it("reveals email verification only when the profile email changes", async () => {
    render(<UserInfoSection />);

    expect(screen.queryByLabelText(/Verification code/i)).not.toBeInTheDocument();
    await userEvent.clear(screen.getByLabelText(/Email/i));
    await userEvent.type(
      screen.getByLabelText(/Email/i),
      "alice+new@example.com",
    );

    expect(screen.getByLabelText(/Verification code/i)).toBeInTheDocument();
  });

  it("creates API tokens through the existing mutation contract", async () => {
    render(<ApiTokenSection />);

    await userEvent.type(screen.getByLabelText(/Token name/i), "CI token");
    await userEvent.type(screen.getByLabelText(/Expires in/i), "7");
    await userEvent.click(
      screen.getByRole("button", { name: /Create token/i }),
    );

    expect(mockCreateMutate).toHaveBeenCalledWith(
      {
        name: "CI token",
        expires_in_days: 7,
        webdav_enabled: true,
        webdav_read_only: false,
        webdav_root_folder_id: null,
      },
      expect.objectContaining({
        onError: expect.any(Function),
        onSuccess: expect.any(Function),
      }),
    );
  });

  it("keeps the custom WebDAV permission controls wired to token creation", async () => {
    render(<ApiTokenSection />);

    expect(screen.getByTestId("webdav-enabled-option")).toHaveClass(
      "has-[:checked]:bg-[var(--settings-secondary-bg)]",
    );
    expect(screen.getByTestId("webdav-readonly-option")).toHaveClass(
      "has-[:checked]:bg-[var(--settings-secondary-bg)]",
    );

    await userEvent.click(screen.getByLabelText(/WebDAV read-only/i));
    await userEvent.type(screen.getByLabelText(/Token name/i), "Read token");
    await userEvent.type(screen.getByLabelText(/Expires in/i), "30");
    await userEvent.click(
      screen.getByRole("button", { name: /Create token/i }),
    );

    expect(mockCreateMutate).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "Read token",
        expires_in_days: 30,
        webdav_enabled: true,
        webdav_read_only: true,
      }),
      expect.objectContaining({
        onError: expect.any(Function),
        onSuccess: expect.any(Function),
      }),
    );
  });

  it("opens delete confirmation before deleting an existing token", async () => {
    const token: ApiToken = {
      id: "tok-1",
      name: "Deploy",
      created_at: "2026-05-04T00:00:00Z",
      last_used_at: null,
      expires_at: null,
      webdav_enabled: true,
      webdav_read_only: false,
      webdav_root_folder_id: null,
    };
    vi.mocked(useApiTokens).mockReturnValue(asApiTokensQuery([token]));

    render(<ApiTokenSection />);
    await userEvent.click(screen.getByRole("button", { name: /Delete/i }));

    expect(screen.getByText(/Delete this token/i)).toBeInTheDocument();
    expect(mockDeleteMutate).not.toHaveBeenCalled();
  });

  it("uses semantic settings tokens for error input states", () => {
    expect(settingsInputClass(true)).toContain("settings-form-error");
    expect(settingsInputClass(false)).toContain("settings-form-input-border");
  });

  it("returns to the previous route instead of hardcoding files home", async () => {
    render(
      <MemoryRouter
        initialEntries={["/files?folder=folder-1", "/settings"]}
        initialIndex={1}
      >
        <Routes>
          <Route path="/files" element={<LocationProbe />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    await userEvent.click(screen.getByRole("button", { name: /Back/i }));

    expect(screen.getByTestId("location")).toHaveTextContent(
      "/files?folder=folder-1",
    );
  });

  it("falls back to files home when there is no previous app route", async () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/files" element={<LocationProbe />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    await userEvent.click(screen.getByRole("button", { name: /Back/i }));

    expect(screen.getByTestId("location")).toHaveTextContent("/files");
  });

  it("does not render Settings quick nav links", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.queryByText("Quick nav")).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Jump to Account/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Jump to Storage/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Jump to Appearance/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Jump to Security/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Jump to Tokens/i })).not.toBeInTheDocument();
  });

  it("does not render the Appearance section on the Settings page", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(
      screen.queryByRole("heading", { name: "Appearance" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Classic purple theme/i }),
    ).not.toBeInTheDocument();
  });

  it("keeps the Settings Center header outside the redesigned content groups", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    const cardGrid = screen.getByTestId("settings-card-grid");

    expect(cardGrid).toHaveClass("grid");
    expect(cardGrid).not.toHaveTextContent("Settings Center");
    expect(cardGrid).toHaveTextContent("Account");
    expect(cardGrid).toHaveTextContent("Storage");
    expect(cardGrid).toHaveTextContent("Security");
    expect(cardGrid).toHaveTextContent("WebDAV Access");
    expect(cardGrid).toHaveTextContent("OCR Status");
    expect(cardGrid).toHaveTextContent("API Tokens");
  });

  it("adds Settings visual polish without changing the content layout", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByTestId("settings-hero-panel")).toHaveClass(
      "isolate",
      "transition-[border-color,box-shadow]",
    );

    const settingsCards = screen.getAllByTestId("settings-card-shell");
    expect(settingsCards).toHaveLength(6);
    expect(settingsCards[0]).toHaveClass(
      "group/settings-card",
      "transition-[border-color,box-shadow,transform]",
    );
    expect(screen.getByTestId("settings-card-grid")).toHaveTextContent(
      /Account[\s\S]*Security[\s\S]*WebDAV Access[\s\S]*API Tokens[\s\S]*OCR Status[\s\S]*Storage/,
    );
  });

  it("uses focused Settings layout groups instead of forcing every pair equal height", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    const identityGroup = screen.getByTestId("settings-group-identity");
    const webdavFocus = screen.getByTestId("settings-group-webdav-focus");
    const tokenWorkspace = screen.getByTestId("settings-group-token-workspace");
    const statusGroup = screen.getByTestId("settings-group-status");

    expect(identityGroup).toHaveClass("xl:grid-cols-2", "xl:items-stretch");
    expect(webdavFocus).toHaveClass(
      "xl:[&>section]:p-[clamp(1.35rem,2.8vw,1.75rem)]",
    );
    expect(tokenWorkspace).toHaveClass(
      "xl:[&>section]:p-[clamp(1.35rem,2.8vw,1.75rem)]",
    );
    expect(statusGroup).toHaveClass("lg:grid-cols-2", "lg:items-stretch");
    expect(identityGroup).toHaveTextContent(/Account[\s\S]*Security/);
    expect(webdavFocus).toHaveTextContent("WebDAV Access");
    expect(tokenWorkspace).toHaveTextContent("API Tokens");
    expect(statusGroup).toHaveTextContent(/OCR Status[\s\S]*Storage/);
    expect(
      screen.queryByTestId("settings-card-row-webdav-tokens"),
    ).not.toBeInTheDocument();
  });

  it("aligns first-row form actions and stretches the bottom status row", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByTestId("settings-account-column")).toHaveClass(
      "[&>section]:h-full",
    );
    expect(screen.getByTestId("settings-security-column")).toHaveClass(
      "[&>section]:h-full",
    );
    expect(screen.getByTestId("settings-account-actions")).toHaveClass(
      "mt-auto",
    );
    expect(screen.getByTestId("settings-security-actions")).toHaveClass(
      "mt-auto",
    );
    expect(screen.getByTestId("settings-ocr-column")).toHaveClass(
      "[&>section]:h-full",
    );
    expect(screen.getByTestId("settings-storage-column")).toHaveClass(
      "[&>section]:h-full",
    );
  });

  it("returns home only from the title icon", async () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/files" element={<LocationProbe />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    await userEvent.click(screen.getByRole("button", { name: /Go to files home/i }));

    expect(screen.getByTestId("location")).toHaveTextContent("/files");
  });

  it("shows WebDAV access guidance without rendering a real token", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText("WebDAV Access")).toBeInTheDocument();
    expect(screen.getByText("Connection details")).toBeInTheDocument();
    expect(screen.getByText("Credential mapping")).toBeInTheDocument();
    expect(screen.getByText("Setup order")).toBeInTheDocument();
    expect(screen.getByText("Client notes")).toBeInTheDocument();
    expect(screen.getByText("http://localhost:3000/dav")).toBeInTheDocument();
    expect(screen.getByText(/Password maps to an API Token/i)).toBeInTheDocument();
    expect(screen.getByText(/Open your WebDAV client/i)).toBeInTheDocument();
    expect(screen.getByText("Finder")).toBeInTheDocument();
    expect(screen.getByText("rclone")).toBeInTheDocument();
    expect(screen.queryByText("test-token")).not.toBeInTheDocument();
  });

  it("keeps WebDAV panels readable in the full-width focused Settings section", () => {
    render(
      <MemoryRouter initialEntries={["/settings"]}>
        <Routes>
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByTestId("webdav-access-grid")).toHaveClass(
      "grid",
    );
    expect(screen.getByTestId("webdav-guidance-grid")).toHaveClass(
      "lg:grid-cols-3",
    );
    expect(screen.getByTestId("webdav-connection-panel")).not.toHaveClass(
      "h-full",
    );
    expect(screen.getByTestId("webdav-setup-panel")).toHaveClass(
      "h-full",
    );
    expect(screen.getByTestId("webdav-credentials-panel")).toHaveClass(
      "h-full",
    );
    expect(screen.getByTestId("webdav-clients-panel")).toHaveClass(
      "h-full",
    );
  });
});
