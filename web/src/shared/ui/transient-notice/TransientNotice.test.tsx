import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { i18n } from '@/shared/i18n';
import { TransientNotice } from './TransientNotice';

describe('TransientNotice', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    window.matchMedia = vi.fn().mockReturnValue({ matches: true }) as never;
  });
  afterEach(async () => {
    vi.useRealTimers();
    await i18n.changeLanguage('en');
  });

  it('shows the message as a status with a tone', () => {
    render(<TransientNotice message="Could not move" onDismiss={() => {}} />);
    const notice = screen.getByRole('status');
    expect(notice.textContent).toBe('Could not move');
    expect(notice.getAttribute('data-tone')).toBe('error');
  });

  it('draws nothing without a message', () => {
    render(<TransientNotice message={null} onDismiss={() => {}} />);
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('goes away by itself after 6 seconds', () => {
    const onDismiss = vi.fn();
    render(<TransientNotice message="Could not move" onDismiss={onDismiss} />);
    act(() => vi.advanceTimersByTime(5999));
    expect(onDismiss).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(1));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('is dismissed by the close button', () => {
    const onDismiss = vi.fn();
    render(<TransientNotice message="Could not move" onDismiss={onDismiss} />);
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss message' }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('plays the exit before it dismisses when motion is allowed', () => {
    window.matchMedia = vi.fn().mockReturnValue({ matches: false }) as never;
    const onDismiss = vi.fn();
    render(<TransientNotice message="Could not move" onDismiss={onDismiss} />);
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss message' }));
    expect(onDismiss).not.toHaveBeenCalled();
    expect(screen.getByRole('status').getAttribute('data-leaving')).toBe('true');
    act(() => vi.advanceTimersByTime(150));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('labels the close button in Japanese', async () => {
    await i18n.changeLanguage('ja');
    render(<TransientNotice message="x" onDismiss={() => {}} />);
    expect(screen.getByRole('button', { name: 'メッセージを閉じる' })).toBeTruthy();
  });
});
