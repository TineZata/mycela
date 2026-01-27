// Custom Tooltip System - Replaces native browser tooltips with styled ones
(function() {
    'use strict';
    
    let tooltip = null;
    let isPinned = false;
    let autoHideTimer = null;
    let currentTargetElement = null;
    let showTimer = null;
    
    // Create tooltip element
    function createTooltip() {
        tooltip = document.createElement('div');
        tooltip.className = 'custom-tooltip';
        tooltip.style.position = 'absolute';
        tooltip.style.pointerEvents = 'auto';
        tooltip.style.opacity = '0';
        tooltip.style.transition = 'opacity 0.2s ease';
        tooltip.style.zIndex = '10000';
        tooltip.style.cursor = 'pointer';
        document.body.appendChild(tooltip);
        
        // Click to pin/unpin
        tooltip.addEventListener('click', function(e) {
            e.stopPropagation();
            togglePin();
        });
        
        // Keep tooltip visible when hovering over it
        tooltip.addEventListener('mouseenter', function() {
            clearAutoHideTimer();
        });
        
        tooltip.addEventListener('mouseleave', function() {
            if (!isPinned) {
                startAutoHideTimer();
            }
        });
    }
    
    // Toggle pin state
    function togglePin() {
        isPinned = !isPinned;
        const pinIndicator = tooltip.querySelector('.tooltip-pin-indicator');
        
        if (isPinned) {
            tooltip.style.borderColor = 'var(--accent)';
            tooltip.style.borderWidth = '2px';
            if (pinIndicator) {
                pinIndicator.style.opacity = '1';
                pinIndicator.style.color = 'var(--accent)';
            }
            clearAutoHideTimer();
        } else {
            tooltip.style.borderColor = 'rgba(255, 255, 255, 0.2)';
            tooltip.style.borderWidth = '1px';
            if (pinIndicator) {
                pinIndicator.style.opacity = '0.4';
                pinIndicator.style.color = 'rgba(255, 255, 255, 0.7)';
            }
            startAutoHideTimer();
        }
    }
    
    // Start auto-hide timer
    function startAutoHideTimer() {
        clearAutoHideTimer();
        if (!isPinned) {
            autoHideTimer = setTimeout(() => {
                // Double-check tooltip isn't being hovered before hiding
                if (!isPinned && tooltip && !tooltip.matches(':hover')) {
                    tooltip.style.opacity = '0';
                    
                    // Don't restore title attributes yet - they'll be restored on mouseout
                    
                    setTimeout(() => {
                        if (tooltip && !isPinned && !tooltip.matches(':hover')) {
                            tooltip.style.display = 'none';
                        }
                    }, 200);
                }
            }, 3000); // Increased to 3 seconds
        }
    }
    
    // Clear auto-hide timer
    function clearAutoHideTimer() {
        if (autoHideTimer) {
            clearTimeout(autoHideTimer);
            autoHideTimer = null;
        }
    }
    
    // Clear show timer
    function clearShowTimer() {
        if (showTimer) {
            clearTimeout(showTimer);
            showTimer = null;
        }
    }
    
    // Position tooltip
    function positionTooltip(e, targetElement) {
        if (!tooltip) return;
        
        const padding = 10;
        
        // Get the element's bounding rectangle
        const elementRect = targetElement.getBoundingClientRect();
        
        // Get tooltip dimensions
        const tooltipRect = tooltip.getBoundingClientRect();
        const viewportWidth = window.innerWidth;
        
        // Position below the widget
        let left = elementRect.left;
        let top = elementRect.bottom + padding;
        
        // Horizontal positioning - keep within viewport
        if (left + tooltipRect.width > viewportWidth) {
            // Align to right edge of element if tooltip is too wide
            left = Math.max(padding, elementRect.right - tooltipRect.width);
        }
        
        // Ensure we don't go off the left edge
        left = Math.max(padding, left);
        
        tooltip.style.left = left + window.scrollX + 'px';
        tooltip.style.top = top + window.scrollY + 'px';
    }
    
    // Show tooltip
    function showTooltip(e) {
        // Search up the DOM tree for an element with title, data-tooltip, or data-original-title
        let element = e.target;
        let title = null;
        
        while (element && !title) {
            title = element.getAttribute('title') || 
                    element.getAttribute('data-tooltip') || 
                    element.getAttribute('data-original-title');
            if (title) break;
            element = element.parentElement;
        }
        
        if (!title || !element) return;
        e.target = element; // Use the element with the title
        currentTargetElement = element; // Store for repositioning
        
        // Store original title if not already stored
        if (element.hasAttribute('title')) {
            element.setAttribute('data-original-title', title);
            element.removeAttribute('title');
        }
        
        if (!tooltip) createTooltip();
        
        // Create content structure with pin indicator
        tooltip.innerHTML = '';
        
        const contentDiv = document.createElement('div');
        contentDiv.textContent = title;
        
        // Material Design pin icon (push pin)
        const pinIndicator = document.createElement('div');
        pinIndicator.className = 'tooltip-pin-indicator';
        pinIndicator.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M16 9V4h1c.55 0 1-.45 1-1s-.45-1-1-1H7c-.55 0-1 .45-1 1s.45 1 1 1h1v5c0 1.66-1.34 3-3 3v2h5.97v7l1 1 1-1v-7H19v-2c-1.66 0-3-1.34-3-3z"/></svg>';
        pinIndicator.style.position = 'absolute';
        pinIndicator.style.top = '4px';
        pinIndicator.style.right = '4px';
        pinIndicator.style.opacity = '0.4';
        pinIndicator.style.cursor = 'pointer';
        pinIndicator.style.color = 'rgba(255, 255, 255, 0.7)';
        pinIndicator.style.transition = 'opacity 0.2s, color 0.2s';
        pinIndicator.style.padding = '2px';
        pinIndicator.style.display = 'flex';
        pinIndicator.style.alignItems = 'center';
        pinIndicator.style.justifyContent = 'center';
        
        // Hover effect for pin icon
        pinIndicator.addEventListener('mouseenter', function() {
            if (!isPinned) {
                this.style.opacity = '0.8';
            }
        });
        pinIndicator.addEventListener('mouseleave', function() {
            if (!isPinned) {
                this.style.opacity = '0.4';
            }
        });
        
        tooltip.appendChild(pinIndicator);
        tooltip.appendChild(contentDiv);
        
        tooltip.style.opacity = '0';
        tooltip.style.display = 'block';
        tooltip.style.borderColor = 'rgba(255, 255, 255, 0.2)';
        tooltip.style.borderWidth = '1px';
        isPinned = false;
        
        // Position after display to get correct dimensions
        requestAnimationFrame(() => {
            positionTooltip(e, element);
            tooltip.style.opacity = '1';
            startAutoHideTimer();
        });
    }
    
    // Hide tooltip
    function hideTooltip(e) {
        if (!tooltip || isPinned) return;
        
        // Don't hide if tooltip is being hovered
        if (tooltip.matches(':hover')) return;
        
        clearAutoHideTimer();
        tooltip.style.opacity = '0';
        setTimeout(() => {
            if (tooltip && !isPinned && !tooltip.matches(':hover')) {
                tooltip.style.display = 'none';
            }
        }, 200);
        
        // Search up for element with data-original-title
        let element = e.target;
        while (element) {
            const originalTitle = element.getAttribute('data-original-title');
            if (originalTitle) {
                element.setAttribute('title', originalTitle);
                element.removeAttribute('data-original-title');
                break;
            }
            element = element.parentElement;
        }
    }
    
    // Update tooltip position on mouse move (not needed for fixed positioning, but keep for compatibility)
    function updateTooltip(e) {
        // Tooltip is now fixed to widget position, no need to update on mouse move
    }
    
    // Initialize tooltips for all elements with title or data-tooltip attributes
    function initTooltips() {
        let currentTooltipElement = null;
        
        document.addEventListener('mouseover', function(e) {
            const target = e.target.closest('[title], [data-tooltip], [data-original-title]');
            if (target && target !== currentTooltipElement) {
                currentTooltipElement = target;
                clearAutoHideTimer();
                clearShowTimer();
                
                // Immediately store and remove title to prevent native tooltip
                const title = target.getAttribute('title');
                if (title) {
                    target.setAttribute('data-original-title', title);
                    target.removeAttribute('title');
                }
                
                // Delay showing tooltip by 1.5 seconds
                showTimer = setTimeout(() => {
                    showTooltip({ target, clientX: e.clientX, clientY: e.clientY });
                }, 1500);
            }
        });
        
        document.addEventListener('mousemove', function(e) {
            const target = e.target.closest('[title], [data-tooltip], [data-original-title]');
            // If mouse moves while waiting to show tooltip, reset the timer
            if (target === currentTooltipElement && showTimer) {
                clearShowTimer();
                showTimer = setTimeout(() => {
                    showTooltip({ target, clientX: e.clientX, clientY: e.clientY });
                }, 1500);
            }
        });
        
        document.addEventListener('mouseout', function(e) {
            const target = e.target.closest('[title], [data-tooltip], [data-original-title]');
            // Don't hide if mouse is moving to the tooltip itself
            if (target === currentTooltipElement && 
                !target.contains(e.relatedTarget) && 
                e.relatedTarget !== tooltip && 
                !tooltip.contains(e.relatedTarget) &&
                !isPinned) {
                
                clearShowTimer(); // Cancel pending show
                
                // Restore title attributes when leaving widget
                const originalTitle = target.getAttribute('data-original-title');
                if (originalTitle) {
                    target.setAttribute('title', originalTitle);
                    target.removeAttribute('data-original-title');
                }
                currentTooltipElement = null;
                
                // Give time to move to tooltip, then check if it's being hovered
                setTimeout(() => {
                    if (tooltip && !tooltip.matches(':hover') && !isPinned) {
                        // Start auto-hide instead of hiding immediately
                        startAutoHideTimer();
                    }
                }, 300);
            }
        });
        
        // Don't update position on mouse move - tooltip stays where it was initially positioned
        // document.addEventListener('mousemove', function(e) {
        //     if (tooltip && tooltip.style.opacity === '1' && !isPinned) {
        //         updateTooltip(e);
        //     }
        // });
        
        // Close pinned tooltip when clicking outside
        document.addEventListener('click', function(e) {
            if (tooltip && isPinned && !tooltip.contains(e.target)) {
                isPinned = false;
                tooltip.style.borderColor = 'rgba(255, 255, 255, 0.2)';
                tooltip.style.borderWidth = '1px';
                tooltip.style.opacity = '0';
                
                // Restore title attribute to all elements with data-original-title
                document.querySelectorAll('[data-original-title]').forEach(el => {
                    const originalTitle = el.getAttribute('data-original-title');
                    if (originalTitle) {
                        el.setAttribute('title', originalTitle);
                        el.removeAttribute('data-original-title');
                    }
                });
                
                currentTooltipElement = null;
                
                setTimeout(() => {
                    if (tooltip && !isPinned) tooltip.style.display = 'none';
                }, 200);
            }
        });
    }
    
    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initTooltips);
    } else {
        initTooltips();
    }
})();
