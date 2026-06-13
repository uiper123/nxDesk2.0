import React from "react";

export interface IconProps {
  size?: number;
  className?: string;
}

const base = (size: number) => ({
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.75,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true as const,
});

export const IconShield: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M12 3l7 3v5c0 4.4-2.8 8.4-7 10-4.2-1.6-7-5.6-7-10V6l7-3z" />
  </svg>
);

export const IconGrid: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <rect x="4" y="4" width="7" height="7" rx="1" />
    <rect x="13" y="4" width="7" height="7" rx="1" />
    <rect x="4" y="13" width="7" height="7" rx="1" />
    <rect x="13" y="13" width="7" height="7" rx="1" />
  </svg>
);

export const IconMonitor: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <rect x="3" y="5" width="18" height="12" rx="1.5" />
    <path d="M9 21h6M12 17v4" />
  </svg>
);

export const IconSliders: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M5 4v6M5 14v6M12 4v2M12 10v10M19 4v10M19 18v2" />
    <path d="M3 10h4M10 6h4M17 14h4" />
  </svg>
);

export const IconList: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M9 6h11M9 12h11M9 18h11" />
    <path d="M4 6h.01M4 12h.01M4 18h.01" />
  </svg>
);

export const IconGear: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1.03 1.56V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1.11-1.56 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.56-1.03H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.56-1.11 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.08a1.7 1.7 0 0 0 1.03-1.56V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1.03 1.56h.08a1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.08a1.7 1.7 0 0 0 1.56 1.03H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.56 1.03z" />
  </svg>
);

export const IconUser: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <circle cx="12" cy="8" r="4" />
    <path d="M4 21c0-4 3.6-6 8-6s8 2 8 6" />
  </svg>
);

export const IconSearch: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <circle cx="11" cy="11" r="7" />
    <path d="M21 21l-4.3-4.3" />
  </svg>
);

export const IconPlus: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M12 5v14M5 12h14" />
  </svg>
);

export const IconClose: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
);

export const IconCheck: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M5 13l4 4L19 7" />
  </svg>
);

export const IconRefresh: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M21 12a9 9 0 1 1-2.64-6.36" />
    <path d="M21 3v6h-6" />
  </svg>
);

export const IconFolder: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
  </svg>
);

export const IconApps: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <circle cx="6" cy="6" r="2" />
    <circle cx="12" cy="6" r="2" />
    <circle cx="18" cy="6" r="2" />
    <circle cx="6" cy="12" r="2" />
    <circle cx="12" cy="12" r="2" />
    <circle cx="18" cy="12" r="2" />
    <circle cx="6" cy="18" r="2" />
    <circle cx="12" cy="18" r="2" />
    <circle cx="18" cy="18" r="2" />
  </svg>
);

export const IconZoom: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <circle cx="11" cy="11" r="7" />
    <path d="M21 21l-4.3-4.3M8 11h6M11 8v6" />
  </svg>
);

export const IconCompass: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <circle cx="12" cy="12" r="9" />
    <path d="M15.5 8.5l-2.1 5-5 2.1 2.1-5 5-2.1z" />
  </svg>
);

export const IconInfo: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 8h.01M12 11v5" />
  </svg>
);

export const IconLink: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M10 14a5 5 0 0 0 7.07 0l2.12-2.12a5 5 0 0 0-7.07-7.07L11 5.93" />
    <path d="M14 10a5 5 0 0 0-7.07 0l-2.12 2.12a5 5 0 0 0 7.07 7.07L13 18.07" />
  </svg>
);

export const IconExpand: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M8 21H5a2 2 0 0 1-2-2v-3M16 21h3a2 2 0 0 0 2-2v-3" />
  </svg>
);

export const IconDownload: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M12 3v12M7 10l5 5 5-5" />
    <path d="M4 21h16" />
  </svg>
);

export const IconRocket: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M12 16c-2 0-4-2-4-4 0-4 3-8.5 4-9 1 .5 4 5 4 9 0 2-2 4-4 4z" />
    <path d="M8 12l-3 3 1 2 3-1M16 12l3 3-1 2-3-1M12 16v5" />
  </svg>
);

export const IconTrash: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M10 11v6M14 11v6" />
  </svg>
);

export const IconEdit: React.FC<IconProps> = ({ size = 18, className }) => (
  <svg {...base(size)} className={className}>
    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7M18.5 2.5a2.121 2.121 0 1 1 3 3L12 15l-4 1 1-4z" />
  </svg>
);
