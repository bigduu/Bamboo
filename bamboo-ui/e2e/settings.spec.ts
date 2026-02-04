import { test, expect } from "@playwright/test";

test.describe("Settings Functionality", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:3000/settings/server");
  });

  test("should display server settings page", async ({ page }) => {
    await expect(page.locator("h1, h2").filter({ hasText: /Server|服务器/ })).toBeVisible();
    await expect(page.locator("input#api-url")).toBeVisible();
    await expect(page.locator("input#ws-url")).toBeVisible();
  });

  test("should update API URL", async ({ page }) => {
    const newUrl = "http://new-api.com:8080";
    
    await page.fill("input#api-url", newUrl);
    await page.blur("input#api-url");
    
    // Verify the value is updated
    await expect(page.locator("input#api-url")).toHaveValue(newUrl);
  });

  test("should update WebSocket URL", async ({ page }) => {
    const newUrl = "ws://new-ws.com:9000";
    
    await page.fill("input#ws-url", newUrl);
    await page.blur("input#ws-url");
    
    await expect(page.locator("input#ws-url")).toHaveValue(newUrl);
  });

  test("should test connection successfully", async ({ page }) => {
    // Mock successful connection
    await page.route("**/health", async (route) => {
      await route.fulfill({
        status: 200,
        body: JSON.stringify({ status: "ok" }),
      });
    });
    
    await page.click("button:has-text('Test')");
    
    // Should show success message
    await expect(page.locator("text=Connected")).toBeVisible({ timeout: 10000 });
  });

  test("should handle connection test failure", async ({ page }) => {
    // Mock failed connection
    await page.route("**/health", async (route) => {
      await route.fulfill({
        status: 500,
        body: JSON.stringify({ error: "Internal Server Error" }),
      });
    });
    
    await page.click("button:has-text('Test')");
    
    // Should show error message
    await expect(page.locator("text=Connection failed")).toBeVisible({ timeout: 10000 });
  });

  test("should navigate to model settings", async ({ page }) => {
    await page.click("a[href*='settings']");
    
    // Should be on settings page
    await expect(page).toHaveURL(/settings/);
  });

  test("should update model configuration", async ({ page }) => {
    // Navigate to model settings if not already there
    await page.goto("http://localhost:3000/settings");
    
    const newModel = "gpt-4";
    await page.fill("input[name='model']", newModel);
    await page.blur("input[name='model']");
    
    await expect(page.locator("input[name='model']")).toHaveValue(newModel);
  });

  test("should update system prompt", async ({ page }) => {
    await page.goto("http://localhost:3000/settings");
    
    const newPrompt = "You are a helpful coding assistant.";
    await page.fill("textarea[name='systemPrompt']", newPrompt);
    await page.blur("textarea[name='systemPrompt']");
    
    await expect(page.locator("textarea[name='systemPrompt']")).toHaveValue(newPrompt);
  });

  test("should reset settings to defaults", async ({ page }) => {
    // Change a setting
    await page.fill("input#api-url", "http://custom.com");
    
    // Click reset button
    await page.click("button:has-text('Reset')");
    
    // Confirm reset
    await page.click("button:has-text('Confirm')");
    
    // Verify default value is restored
    await expect(page.locator("input#api-url")).toHaveValue("http://localhost:3000");
  });

  test("should persist settings after page reload", async ({ page }) => {
    const newUrl = "http://persisted.com:3000";
    
    // Update setting
    await page.fill("input#api-url", newUrl);
    await page.blur("input#api-url");
    
    // Reload page
    await page.reload();
    
    // Verify setting persisted
    await expect(page.locator("input#api-url")).toHaveValue(newUrl);
  });
});
