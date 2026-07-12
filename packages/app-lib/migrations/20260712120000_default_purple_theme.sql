-- ByteLauncher: default fresh installs to the Purple theme.
-- Gated on `onboarded = 0` so this only affects brand-new installs (migrations
-- run before onboarding). Existing users have onboarded = 1 and keep whatever
-- theme they previously chose.
UPDATE settings SET theme = 'purple' WHERE theme = 'dark' AND onboarded = 0;
