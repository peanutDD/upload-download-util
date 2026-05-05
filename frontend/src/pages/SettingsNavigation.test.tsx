import type { ReactNode } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import Settings from "./Settings";

vi.mock("../components/layout/PageLayout", () => ({
  default: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock("../components/settings/UserInfoSection", () => ({ default: () => <section /> }));
vi.mock("../components/settings/StorageUsageSection", () => ({ default: () => <section /> }));
vi.mock("../components/settings/ThemeSection", () => ({ default: () => <section /> }));
vi.mock("../components/settings/PasswordChangeSection", () => ({ default: () => <section /> }));
vi.mock("../components/settings/ApiTokenSection", () => ({ default: () => <section /> }));

vi.mock("../store/authStore", () => ({
  useAuthStore: (selector: (state: unknown) => unknown) =>
    selector({
      user: { username: "alice" },
      clearAuth: vi.fn(),
    }),
}));

vi.mock("../hooks/useStorageUsage", () => ({
  useStorageUsage: () => ({
    data: { file_count: 12, total_size: 4096 },
  }),
}));

vi.mock("../hooks/useApiTokens", () => ({
  useApiTokens: () => ({ data: [] }),
}));

function LocationProbe() {
  const location = useLocation();
  return (
    <output data-testid="location">
      {location.pathname}{location.search}
    </output>
  );
}

describe("Settings navigation", () => {
  it("returns to the previous history entry instead of hardcoding the files root", async () => {
    render(
      <MemoryRouter initialEntries={["/files?folder=folder-1", "/settings"]} initialIndex={1}>
        <Settings />
        <LocationProbe />
      </MemoryRouter>,
    );

    await userEvent.click(screen.getByRole("button", { name: /^Back$/i }));

    expect(screen.getByTestId("location")).toHaveTextContent(
      "/files?folder=folder-1",
    );
  });
});
