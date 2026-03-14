import { test, expect } from '@playwright/test';

test.describe('Universal Tuner Card Design Verification', () => {
  test('should match Golden Ratio dimensions and Thin Glass aesthetic', async ({ page }) => {
    await page.goto('/');

    const tunerCard = page.locator('.glass-card').first();
    await expect(tunerCard).toBeVisible();

    // Verify Golden Ratio (1000x618)
    const box = await tunerCard.boundingBox();
    if (box) {
      expect(Math.round(box.width)).toBe(1000);
      expect(Math.round(box.height)).toBe(618);
      const ratio = box.width / box.height;
      expect(ratio).toBeCloseTo(1.618, 1);
    }

    // Verify Glass Aesthetic (Computed Styles)
    const glassStyle = await tunerCard.evaluate((el) => {
      const style = window.getComputedStyle(el);
      return {
        backdropBlur: style.backdropFilter,
        backgroundColor: style.backgroundColor,
        borderWidth: style.borderWidth
      };
    });

    // Check for backdrop blur (any value exists)
    expect(glassStyle.backdropBlur).not.toBe('none');
    
    // Check for visible frequency
    await expect(page.getByText('102.100')).toBeVisible();
    await expect(page.getByText('[ NO SOURCE CONNECTED ]')).toBeVisible();
    
    // Check for forensic integrity markers
    await expect(page.getByText('[UNWIRED]')).toBeVisible();
  });
});
