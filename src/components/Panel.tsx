import { ReactNode } from 'react';

interface PanelProps {
  title?: string;
  children: ReactNode;
  className?: string;
  headerActions?: ReactNode;
}

export function Panel({ title, children, className = '', headerActions }: PanelProps) {
  return (
    <div className={`panel ${className}`}>
      {title && (
        <div className="panel-header flex items-center justify-between">
          <h3 className="text-sm font-medium text-secondary-100">{title}</h3>
          {headerActions && (
            <div className="flex items-center gap-1">
              {headerActions}
            </div>
          )}
        </div>
      )}
      <div className="panel-content">
        {children}
      </div>
    </div>
  );
}