import { test, expect } from "@playwright/test";

test.describe("Chat Functionality", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the chat page before each test
    await page.goto("http://localhost:3000");
  });

  test("should display chat interface", async ({ page }) => {
    // Check that main chat elements are visible
    await expect(page.locator("[data-testid='chat-container']")).toBeVisible();
    await expect(page.locator("[data-testid='message-list']")).toBeVisible();
    await expect(page.locator("[data-testid='message-input']")).toBeVisible();
  });

  test("should create a new session", async ({ page }) => {
    // Click new session button
    await page.click("[data-testid='new-session-button']");
    
    // Check that a new session appears in the list
    await expect(page.locator("[data-testid='session-list']")).toContainText("新会话");
  });

  test("should send a message", async ({ page }) => {
    // Create a new session first
    await page.click("[data-testid='new-session-button']");
    
    // Type and send a message
    await page.fill("[data-testid='message-input']", "Hello, AI!");
    await page.keyboard.press("Enter");
    
    // Check that the message appears in the chat
    await expect(page.locator("[data-testid='message-list']")).toContainText("Hello, AI!");
  });

  test("should receive AI response", async ({ page }) => {
    // Create a new session
    await page.click("[data-testid='new-session-button']");
    
    // Send a message
    await page.fill("[data-testid='message-input']", "What is the weather?");
    await page.keyboard.press("Enter");
    
    // Wait for AI response (with timeout)
    await expect(page.locator("[data-testid='message-assistant']")).toBeVisible({ timeout: 30000 });
  });

  test("should switch between sessions", async ({ page }) => {
    // Create two sessions
    await page.click("[data-testid='new-session-button']");
    await page.click("[data-testid='new-session-button']");
    
    // Get session IDs
    const sessions = await page.locator("[data-testid^='session-']").all();
    expect(sessions.length).toBeGreaterThanOrEqual(2);
    
    // Click on first session
    await sessions[0].click();
    
    // Verify it's marked as active
    await expect(sessions[0]).toHaveAttribute("data-active", "true");
  });

  test("should maintain separate message histories per session", async ({ page }) => {
    // Create first session and send message
    await page.click("[data-testid='new-session-button']");
    await page.fill("[data-testid='message-input']", "Message in session 1");
    await page.keyboard.press("Enter");
    
    // Create second session and send message
    await page.click("[data-testid='new-session-button']");
    await page.fill("[data-testid='message-input']", "Message in session 2");
    await page.keyboard.press("Enter");
    
    // Switch back to first session
    const sessions = await page.locator("[data-testid^='session-']").all();
    await sessions[0].click();
    
    // Verify first session's message is shown
    await expect(page.locator("[data-testid='message-list']")).toContainText("Message in session 1");
    await expect(page.locator("[data-testid='message-list']")).not.toContainText("Message in session 2");
  });

  test("should show loading state while sending", async ({ page }) => {
    await page.click("[data-testid='new-session-button']");
    
    await page.fill("[data-testid='message-input']", "Test message");
    await page.keyboard.press("Enter");
    
    // Check for loading indicator
    await expect(page.locator("[data-testid='loading-indicator']")).toBeVisible();
  });

  test("should handle empty messages", async ({ page }) => {
    await page.click("[data-testid='new-session-button']");
    
    // Try to send empty message
    await page.keyboard.press("Enter");
    
    // No message should be added
    const messages = await page.locator("[data-testid='message-user']").count();
    expect(messages).toBe(0);
  });

  test("should delete a session", async ({ page }) => {
    // Create a session
    await page.click("[data-testid='new-session-button']");
    
    // Get initial session count
    const initialCount = await page.locator("[data-testid^='session-']").count();
    
    // Delete the session
    await page.click("[data-testid='delete-session-button']");
    await page.click("[data-testid='confirm-delete']");
    
    // Verify session is removed
    await expect(page.locator("[data-testid^='session-']")).toHaveCount(initialCount - 1);
  });
});
