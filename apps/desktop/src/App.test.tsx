import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";

afterEach(cleanup);

describe("SupportKit desktop workflow", () => {
  it("runs the synthetic browser flow through findings and privacy review", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(
      screen.getByRole("heading", {
        name: "Turn system evidence into a support case.",
      }),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Start a diagnostic" }),
    );
    expect(
      screen.getByRole("heading", { name: "New diagnostic" }),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("radio", { name: /Network or internet problems/ }),
    );
    await user.click(
      screen.getByRole("button", { name: "Run read-only scan" }),
    );
    expect(
      await screen.findByRole(
        "heading",
        { name: "Findings" },
        { timeout: 3_000 },
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("System disk is critically full"),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Review privacy changes" }),
    );
    expect(
      screen.getByRole("heading", { name: "Privacy review" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Transformed fields")).toBeInTheDocument();
  });

  it("redirects protected navigation when no case is active", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Export" }));
    expect(
      screen.getByRole("heading", { name: "New diagnostic" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Run a diagnostic or reopen a local case first."),
    ).toBeInTheDocument();
  });
});
