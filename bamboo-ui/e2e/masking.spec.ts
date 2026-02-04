import { test, expect } from "@playwright/test";

test.describe("Masking Configuration", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:3000/settings/masking");
  });

  test("should display masking settings page", async ({ page }) => {
    await expect(page.locator("h1, h2").filter({ hasText: /Masking|脱敏/ })).toBeVisible();
    await expect(page.locator("[data-testid='masking-enabled']")).toBeVisible();
  });

  test("should toggle masking enabled state", async ({ page }) => {
    const toggle = page.locator("[data-testid='masking-enabled']");
    
    // Get initial state
    const initialState = await toggle.isChecked();
    
    // Toggle
    await toggle.click();
    
    // Verify state changed
    await expect(toggle).not.toBeChecked();
    
    // Toggle back
    await toggle.click();
    await expect(toggle).toBeChecked();
  });

  test("should add a new masking rule", async ({ page }) => {
    // Click add rule button
    await page.click("[data-testid='add-rule-button']");
    
    // Fill in rule details
    await page.fill("[data-testid='rule-name-input']", "Email Masking");
    await page.fill("[data-testid='rule-pattern-input']", "\\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Z|a-z]{2,}\\b");
    await page.fill("[data-testid='rule-replacement-input']", "[EMAIL]");
    
    // Save rule
    await page.click("[data-testid='save-rule-button']");
    
    // Verify rule appears in list
    await expect(page.locator("[data-testid='rule-list']")).toContainText("Email Masking");
  });

  test("should edit an existing masking rule", async ({ page }) => {
    // Add a rule first
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Original Name");
    await page.fill("[data-testid='rule-pattern-input']", "test-pattern");
    await page.fill("[data-testid='rule-replacement-input']", "[REPLACED]");
    await page.click("[data-testid='save-rule-button']");
    
    // Edit the rule
    await page.click("[data-testid='edit-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Updated Name");
    await page.click("[data-testid='save-rule-button']");
    
    // Verify updated name
    await expect(page.locator("[data-testid='rule-list']")).toContainText("Updated Name");
    await expect(page.locator("[data-testid='rule-list']")).not.toContainText("Original Name");
  });

  test("should delete a masking rule", async ({ page }) => {
    // Add a rule
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Rule to Delete");
    await page.fill("[data-testid='rule-pattern-input']", "pattern");
    await page.fill("[data-testid='rule-replacement-input']", "[X]");
    await page.click("[data-testid='save-rule-button']");
    
    // Delete the rule
    await page.click("[data-testid='delete-rule-button']");
    await page.click("[data-testid='confirm-delete']");
    
    // Verify rule is removed
    await expect(page.locator("[data-testid='rule-list']")).not.toContainText("Rule to Delete");
  });

  test("should test masking rule", async ({ page }) => {
    // Add email masking rule
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Email");
    await page.fill("[data-testid='rule-pattern-input']", "\\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Z|a-z]{2,}\\b");
    await page.fill("[data-testid='rule-replacement-input']", "[EMAIL]");
    await page.click("[data-testid='save-rule-button']");
    
    // Open test panel
    await page.click("[data-testid='test-masking-button']");
    
    // Enter test text
    await page.fill("[data-testid='test-input']", "Contact me at user@example.com");
    
    // Run test
    await page.click("[data-testid='run-test-button']");
    
    // Verify result
    await expect(page.locator("[data-testid='test-result']")).toContainText("[EMAIL]");
  });

  test("should toggle rule enabled state", async ({ page }) => {
    // Add a rule
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Toggle Test");
    await page.fill("[data-testid='rule-pattern-input']", "pattern");
    await page.fill("[data-testid='rule-replacement-input']", "[X]");
    await page.click("[data-testid='save-rule-button']");
    
    // Toggle rule off
    const toggle = page.locator("[data-testid='rule-enabled-toggle']").first();
    await toggle.click();
    
    // Verify toggle state
    await expect(toggle).not.toBeChecked();
  });

  test("should validate rule pattern", async ({ page }) => {
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Invalid Rule");
    
    // Try to save without pattern
    await page.click("[data-testid='save-rule-button']");
    
    // Should show validation error
    await expect(page.locator("[data-testid='pattern-error']")).toBeVisible();
  });

  test("should reorder rules via drag and drop", async ({ page }) => {
    // Add two rules
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Rule 1");
    await page.fill("[data-testid='rule-pattern-input']", "pattern1");
    await page.fill("[data-testid='rule-replacement-input']", "[1]");
    await page.click("[data-testid='save-rule-button']");
    
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Rule 2");
    await page.fill("[data-testid='rule-pattern-input']", "pattern2");
    await page.fill("[data-testid='rule-replacement-input']", "[2]");
    await page.click("[data-testid='save-rule-button']");
    
    // Get initial order
    const rules = await page.locator("[data-testid='rule-item']").all();
    expect(rules.length).toBe(2);
    
    // Drag second rule to first position
    await rules[1].dragTo(rules[0]);
    
    // Verify order changed
    const newRules = await page.locator("[data-testid='rule-item']").all();
    await expect(newRules[0]).toContainText("Rule 2");
  });

  test("should persist masking config after reload", async ({ page }) => {
    // Add a rule
    await page.click("[data-testid='add-rule-button']");
    await page.fill("[data-testid='rule-name-input']", "Persistent Rule");
    await page.fill("[data-testid='rule-pattern-input']", "persistent");
    await page.fill("[data-testid='rule-replacement-input']", "[P]");
    await page.click("[data-testid='save-rule-button']");
    
    // Reload page
    await page.reload();
    
    // Verify rule still exists
    await expect(page.locator("[data-testid='rule-list']")).toContainText("Persistent Rule");
  });
});
