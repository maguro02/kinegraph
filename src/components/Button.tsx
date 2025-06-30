
import React from 'react';
import { ReactNode } from 'react';

interface ButtonProps {
  children?: ReactNode;
  variant?: 'primary' | 'secondary' | 'ghost' | 'icon' | 'danger';
  size?: 'small' | 'medium' | 'large' | 'sm' | 'md' | 'lg';
  disabled?: boolean;
  onClick?: (e?: React.MouseEvent<HTMLButtonElement>) => void;
  className?: string;
  icon?: ReactNode;
  title?: string;
}

export function Button({
  children,
  variant = 'primary',
  size = 'medium',
  disabled = false,
  onClick,
  className = '',
  icon,
  title,
}: ButtonProps) {
  const baseClasses = 'btn inline-flex items-center justify-center gap-2 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed';
  
  const variantClasses = {
    primary: 'btn-primary',
    secondary: 'btn-secondary',
    ghost: 'btn-ghost',
    icon: 'btn-icon',
    danger: 'btn-danger bg-red-600 text-white hover:bg-red-700 active:bg-red-800 focus:ring-2 focus:ring-red-300',
  };
  
  // Support both old and new size naming conventions
  const normalizeSize = (size: string) => {
    switch (size) {
      case 'small':
      case 'sm':
        return 'sm';
      case 'medium':
      case 'md':
        return 'md';
      case 'large':
      case 'lg':
        return 'lg';
      default:
        return 'md';
    }
  };
  
  const sizeClasses = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  };
  
  const normalizedSize = normalizeSize(size);
  const iconOnlyClasses = variant === 'icon' ? 'p-2' : '';
  
  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${variant !== 'icon' ? sizeClasses[normalizedSize] : iconOnlyClasses} ${className}`}
      disabled={disabled}
      onClick={onClick}
      title={title}
    >
      {icon && <span className="flex-shrink-0">{icon}</span>}
      {variant !== 'icon' && children}
    </button>
  );
}