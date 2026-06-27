import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import FileListFilters from "./FileListFilters";

describe("FileListFilters", () => {
  it("调用 onSearchChange 并显示/隐藏清除按钮", async () => {
    const user = userEvent.setup();
    const handleSearchChange = vi.fn();

    const { rerender, getByPlaceholderText, getByRole } = render(
      <FileListFilters
        search=""
        mimeType=""
        sortBy="created_at_desc"
        onSearchChange={handleSearchChange}
        onMimeTypeChange={() => {}}
        onSortChange={() => {}}
        data-oid="u4rss70"
      />,
    );

    const input = getByPlaceholderText("Search… (Ctrl+K)");
    await user.type(input, "test");

    expect(handleSearchChange).toHaveBeenCalled();

    // 重新渲染，模拟父组件把 search 状态更新为非空
    rerender(
      <FileListFilters
        search="test"
        mimeType=""
        sortBy="created_at_desc"
        onSearchChange={handleSearchChange}
        onMimeTypeChange={() => {}}
        onSortChange={() => {}}
        data-oid="kyehbne"
      />,
    );

    const clearButton = getByRole("button", { name: "Clear search" });
    await user.click(clearButton);

    expect(handleSearchChange).toHaveBeenCalledWith("");
  });

  it("keeps type and sort dropdowns inside the filter container", async () => {
    const user = userEvent.setup();
    const { container, getByRole } = render(
      <FileListFilters
        search=""
        mimeType=""
        sortBy="created_at_desc"
        onSearchChange={() => {}}
        onMimeTypeChange={() => {}}
        onSortChange={() => {}}
      />,
    );

    await user.click(getByRole("button", { name: "Type filter" }));
    const typeMenu = getByRole("menu", { name: "Type options" });
    expect(container).toContainElement(typeMenu);

    await user.click(getByRole("button", { name: "Sort order" }));
    const sortMenu = getByRole("menu", { name: "Sort options" });
    expect(container).toContainElement(sortMenu);
  });

  it("uses an opaque dropdown panel instead of the transparent filter surface", () => {
    const css = readFileSync(
      resolve(__dirname, "FileListFilters.css"),
      "utf8",
    );

    expect(css).toContain("position: absolute;");
    expect(css).not.toContain("position: fixed;");
    expect(css).toContain("--filters-dropdown-surface-bg-top");
    expect(css).toContain("--filters-dropdown-surface-bg-bottom");
  });

  it("raises the transformed toolbar above file content while a dropdown is open", () => {
    const filterCss = readFileSync(
      resolve(__dirname, "FileListFilters.css"),
      "utf8",
    );
    const glassCss = readFileSync(
      resolve(__dirname, "FileListGlass.css"),
      "utf8",
    );

    expect(filterCss).toContain("z-index: var(--filters-dropdown-z);");
    expect(glassCss).toContain("z-index: var(--filters-dropdown-toolbar-z);");
  });
});
